// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::{Seed, StepVotes};

use crate::user::cluster::Cluster;
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use bytes::Buf;

use crate::config;
use dusk_bytes::Serializable;
use node_data::bls::PublicKey;
use node_data::message::{marshal_signable_vote, Header, Message, Payload};
use std::fmt::{self, Display};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

#[derive(Debug)]
pub enum Error {
    VoteSetTooSmall(u8),
    VerificationFailed(dusk_bls12_381_sign::Error),
    EmptyApk,
    InvalidType,
    InvalidStepNum,
}

impl From<dusk_bls12_381_sign::Error> for Error {
    fn from(inner: dusk_bls12_381_sign::Error) -> Self {
        Self::VerificationFailed(inner)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::VoteSetTooSmall(step) => {
                write!(f, "Failed to reach a quorum at step {}", step)
            }
            Error::VerificationFailed(_) => write!(f, "Verification error"),
            Error::EmptyApk => write!(f, "Empty Apk instance"),
            Error::InvalidType => write!(f, "Invalid Type"),
            Error::InvalidStepNum => write!(f, "Invalid step number"),
        }
    }
}

/// verify_agreement performs all three-steps verification of an agreement
/// message. It is intended to be used in a context of tokio::spawn as per that
/// it tries to yield before any CPU-bound operation.
pub async fn verify_agreement(
    msg: Message,
    committees_set: Arc<Mutex<CommitteeSet>>,
    seed: Seed,
) -> Result<(), Error> {
    match msg.payload {
        Payload::Agreement(payload) => {
            msg.header
                .verify_signature(&payload.signature)
                .map_err(|e| {
                    error!(
                        desc = "invalid signature",
                        signature =
                            format!("{:?}", hex::encode(payload.signature)),
                        hdr = format!("{:?}", msg.header),
                    );
                    e
                })?;

            // Verify 1st_reduction step_votes
            verify_step_votes(
                &payload.first_step,
                &committees_set,
                seed,
                &msg.header,
                0,
                config::FIRST_REDUCTION_COMMITTEE_SIZE,
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    desc = "invalid 1st_reduction step_votes",
                    sv = format!("{:?}", payload.first_step),
                    hdr = format!("{:?}", msg.header),
                );
                e
            })?;

            // Verify 2th_reduction step_votes
            verify_step_votes(
                &payload.second_step,
                &committees_set,
                seed,
                &msg.header,
                1,
                config::SECOND_REDUCTION_COMMITTEE_SIZE,
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    desc = "invalid 2th_reduction step_votes",
                    sv = format!("{:?}", payload.second_step),
                    hdr = format!("{:?}", msg.header),
                );
                e
            })?;

            // Verification done
            Ok(())
        }
        _ => Err(Error::InvalidType),
    }
}

pub async fn verify_step_votes(
    sv: &StepVotes,
    committees_set: &Arc<Mutex<CommitteeSet>>,
    seed: Seed,
    hdr: &Header,
    step_offset: u8,
    committee_size: usize,
    enable_quorum_check: bool,
) -> Result<(), Error> {
    if hdr.step == 0 {
        return Err(Error::InvalidStepNum);
    }

    let step = hdr.step - 1 + step_offset;
    let cfg = sortition::Config::new(seed, hdr.round, step, committee_size);

    verify_votes(
        &hdr.block_hash,
        sv.bitset,
        &sv.aggregate_signature.inner(),
        committees_set,
        &cfg,
        enable_quorum_check,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn verify_votes(
    block_hash: &[u8; 32],
    bitset: u64,
    signature: &[u8; 48],
    committees_set: &Arc<Mutex<CommitteeSet>>,
    cfg: &sortition::Config,
    enable_quorum_check: bool,
) -> Result<(), Error> {
    // TODO: This should be refactored into a structure when #1118 issue is
    // implemented
    let sub_committee = {
        let mut guard = committees_set.lock().await;
        let sub_committee = guard.intersect(bitset, cfg);

        if enable_quorum_check {
            let target_quorum = guard.quorum(cfg);
            let total = guard.total_occurrences(&sub_committee, cfg);
            if total < target_quorum {
                tracing::error!(
                    desc = "vote_set_too_small",
                    committee = format!("{:#?}", sub_committee),
                    cfg = format!("{:#?}", cfg),
                    bitset = bitset,
                    target_quorum = target_quorum,
                    total = total,
                );
                Err(Error::VoteSetTooSmall(cfg.step))
            } else {
                Ok(sub_committee)
            }
        } else {
            Ok(sub_committee)
        }
    }?;

    // aggregate public keys

    let apk = sub_committee.aggregate_pks()?;

    // verify signatures
    verify_step_signature(cfg.round, cfg.step, block_hash, apk, signature)?;

    // Verification done
    Ok(())
}

impl Cluster<PublicKey> {
    fn aggregate_pks(&self) -> Result<dusk_bls12_381_sign::APK, Error> {
        let pks: Vec<&dusk_bls12_381_sign::PublicKey> =
            self.iter().map(|(pubkey, _)| pubkey.inner()).collect();

        match pks.split_first() {
            Some((&first, rest)) => {
                let mut apk = dusk_bls12_381_sign::APK::from(first);
                rest.iter().for_each(|&&p| apk.aggregate(&[p]));
                Ok(apk)
            }
            None => Err(Error::EmptyApk),
        }
    }
}

fn verify_step_signature(
    round: u64,
    step: u8,
    block_hash: &[u8; 32],
    apk: dusk_bls12_381_sign::APK,
    signature: &[u8; 48],
) -> Result<(), dusk_bls12_381_sign::Error> {
    // Compile message to verify

    let sig = dusk_bls12_381_sign::Signature::from_bytes(signature)?;
    apk.verify(&sig, marshal_signable_vote(round, step, block_hash).bytes())
}
