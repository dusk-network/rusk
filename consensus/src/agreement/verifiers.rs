// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::marshal_signable_vote;

use node_data::ledger::{Seed, StepVotes};

use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use crate::util::cluster::Cluster;
use bytes::Buf;

use dusk_bytes::Serializable;
use node_data::bls::PublicKey;
use node_data::message::{Header, Message, Payload};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    VoteSetTooSmall,
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

/// verify_agreement performs all three-steps verification of an agreement message. It is intended to be used in a context of tokio::spawn as per that it tries to yield before any CPU-bound operation.
pub async fn verify_agreement(
    msg: Message,
    committees_set: Arc<Mutex<CommitteeSet>>,
    seed: Seed,
) -> Result<(), Error> {
    match msg.payload {
        Payload::Agreement(payload) => {
            msg.header.verify_signature(&payload.signature)?;

            // Verify 1th_reduction step_votes
            verify_step_votes(
                &payload.first_step,
                &committees_set,
                seed,
                &msg.header,
                0,
            )
            .await?;

            // Verify 2th_reduction step_votes
            verify_step_votes(
                &payload.second_step,
                &committees_set,
                seed,
                &msg.header,
                1,
            )
            .await?;

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
) -> Result<(), Error> {
    if hdr.step == 0 {
        return Err(Error::InvalidStepNum);
    }

    let step = hdr.step - 1 + step_offset;
    let cfg = sortition::Config::new(seed, hdr.round, step, 64);

    verify_votes(
        &hdr.block_hash,
        sv.bitset,
        &sv.signature.inner(),
        committees_set,
        &cfg,
    )
    .await
}

pub async fn verify_votes(
    block_hash: &[u8; 32],
    bitset: u64,
    signature: &[u8; 48],
    committees_set: &Arc<Mutex<CommitteeSet>>,
    cfg: &sortition::Config,
) -> Result<(), Error> {
    let sub_committee = {
        // Scoped guard to fetch committee data quickly
        let mut guard = committees_set.lock().await;

        let sub_committee = guard.intersect(bitset, cfg);
        let target_quorum = guard.quorum(cfg);

        if guard.total_occurrences(&sub_committee, cfg) < target_quorum {
            Err(Error::VoteSetTooSmall)
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
