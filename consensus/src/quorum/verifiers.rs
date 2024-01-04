// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::{Seed, StepVotes};
use node_data::StepName;

use crate::commons::{Error, IterCounter};
use crate::user::cluster::Cluster;
use crate::user::committee::{Committee, CommitteeSet};
use crate::user::sortition;
use bytes::Buf;

use crate::config;
use dusk_bytes::Serializable;
use node_data::bls::PublicKey;
use node_data::message::{marshal_signable_vote, Header, Message, Payload};
use tokio::sync::RwLock;
use tracing::error;

/// Performs all three-steps verification of a quorum msg.
pub async fn verify_quorum(
    msg: Message,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
) -> Result<(), Error> {
    //TODO use if let
    match msg.payload {
        Payload::Quorum(payload) => {
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

            // Verify validation
            verify_step_votes(
                &payload.validation,
                committees_set,
                seed,
                &msg.header,
                StepName::Validation,
                config::VALIDATION_COMMITTEE_SIZE,
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    desc = "invalid validation",
                    sv = format!("{:?}", payload.validation),
                    hdr = format!("{:?}", msg.header),
                );
                e
            })?;

            // Verify ratification
            verify_step_votes(
                &payload.ratification,
                committees_set,
                seed,
                &msg.header,
                StepName::Ratification,
                config::RATIFICATION_COMMITTEE_SIZE,
                true,
            )
            .await
            .map_err(|e| {
                error!(
                    desc = "invalid ratification",
                    sv = format!("{:?}", payload.ratification),
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
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    hdr: &Header,
    step_name: StepName,
    committee_size: usize,
    enable_quorum_check: bool,
) -> Result<QuorumResult, Error> {
    if step_name == StepName::Proposal {
        return Err(Error::InvalidStepNum);
    }

    let iteration = hdr.iteration;
    let step = iteration.step_from_name(step_name);
    let generator = committees_set
        .read()
        .await
        .provisioners()
        .get_generator(iteration, seed, hdr.round);

    let cfg = sortition::Config::new(
        seed,
        hdr.round,
        step,
        committee_size,
        Some(generator),
    );

    if committees_set.read().await.get(&cfg).is_none() {
        let _ = committees_set.write().await.get_or_create(&cfg);
    }

    let set = committees_set.read().await;
    let committee = set.get(&cfg).expect("committee to be created");

    verify_votes(
        &hdr.block_hash,
        sv.bitset,
        &sv.aggregate_signature.inner(),
        committee,
        &cfg,
        enable_quorum_check,
    )
}

#[derive(Default)]
pub struct QuorumResult {
    pub total: usize,
    pub target_quorum: usize,
}

impl QuorumResult {
    pub fn quorum_reached(&self) -> bool {
        self.total >= self.target_quorum
    }
}

pub fn verify_votes(
    block_hash: &[u8; 32],
    bitset: u64,
    signature: &[u8; 48],
    committee: &Committee,
    cfg: &sortition::Config,
    enable_quorum_check: bool,
) -> Result<QuorumResult, Error> {
    let sub_committee = committee.intersect(bitset);

    let total = committee.total_occurrences(&sub_committee);
    let target_quorum = if block_hash == &[0u8; 32] {
        committee.nil_quorum()
    } else {
        committee.quorum()
    };

    let quorum_result = QuorumResult {
        total,
        target_quorum,
    };

    if enable_quorum_check && !quorum_result.quorum_reached() {
        tracing::error!(
            desc = "vote_set_too_small",
            committee = format!("{:#?}", sub_committee),
            cfg = format!("{:#?}", cfg),
            bitset,
            target_quorum,
            total,
        );
        return Err(Error::VoteSetTooSmall(cfg.step));
    }

    // If bitset=0 this means that we are checking for failed iteration
    // certificates. If a winning certificate is checked with bitset=0 it will
    // fail to pass the quorum and results in VoteSetTooSmall.
    // FIXME: Anyway this should be handled properly, maybe with a different
    // function
    if bitset > 0 {
        // aggregate public keys
        let apk = sub_committee.aggregate_pks()?;

        // verify signatures
        verify_step_signature(cfg.round, cfg.step, block_hash, apk, signature)?;
    }
    // Verification done
    Ok(quorum_result)
}

impl Cluster<PublicKey> {
    fn aggregate_pks(&self) -> Result<dusk_bls12_381_sign::APK, Error> {
        let pks: Vec<_> =
            self.iter().map(|(pubkey, _)| *pubkey.inner()).collect();

        match pks.split_first() {
            Some((first, rest)) => {
                let mut apk = dusk_bls12_381_sign::APK::from(first);
                apk.aggregate(rest);
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
