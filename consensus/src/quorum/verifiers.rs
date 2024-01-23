// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::bls::PublicKey;
use node_data::ledger::{Seed, StepVotes};
use node_data::message::payload::{Quorum, Vote};
use node_data::message::{ConsensusHeader, ConsensusMessage, ConsensusMsgType};
use node_data::{Serializable, StepName};

use crate::commons::Error;
use crate::user::cluster::Cluster;
use crate::user::committee::{Committee, CommitteeSet};
use crate::user::sortition;

use dusk_bytes::Serializable as BytesSerializable;
use tokio::sync::RwLock;
use tracing::error;

/// Performs all three-steps verification of a quorum msg.
pub async fn verify_quorum(
    quorum: &Quorum,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
) -> Result<(), Error> {
    quorum.verify_signature().map_err(|e| {
        error!(
            desc = "invalid signature",
            signature = hex::encode(quorum.header.signature.inner()),
            hdr = ?quorum.header,
        );
        e
    })?;

    // Verify validation
    verify_step_votes(
        &quorum.header,
        &quorum.vote,
        &quorum.validation,
        committees_set,
        seed,
        StepName::Validation,
    )
    .await
    .map_err(|e| {
        error!(
            desc = "invalid validation",
            sv = ?quorum.validation,
            hdr = ?quorum.header,
        );
        e
    })?;

    // Verify ratification
    verify_step_votes(
        &quorum.header,
        &quorum.vote,
        &quorum.ratification,
        committees_set,
        seed,
        StepName::Ratification,
    )
    .await
    .map_err(|e| {
        error!(
            desc = "invalid ratification",
            sv = ?quorum.ratification,
            hdr = ?quorum.header,
        );
        e
    })?;

    Ok(())
}

pub async fn verify_step_votes(
    header: &ConsensusHeader,
    vote: &Vote,
    sv: &StepVotes,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    step_name: StepName,
) -> Result<QuorumResult, Error> {
    let round = header.round;
    let iteration = header.iteration;
    // ConsensusMsgType cannot be taken from header, since we can receive header
    // from different messages (like Quorum)
    let msg_type = match step_name {
        StepName::Proposal => return Err(Error::InvalidStepNum),
        StepName::Validation => ConsensusMsgType::Validation,
        StepName::Ratification => ConsensusMsgType::Ratification,
    };

    let generator = committees_set
        .read()
        .await
        .provisioners()
        .get_generator(iteration, seed, round);

    let cfg = sortition::Config::new(
        seed,
        round,
        iteration,
        step_name,
        Some(generator),
    );

    if committees_set.read().await.get(&cfg).is_none() {
        let _ = committees_set.write().await.get_or_create(&cfg);
    }

    let set = committees_set.read().await;
    let committee = set.get(&cfg).expect("committee to be created");

    verify_votes(
        header,
        msg_type,
        vote,
        sv.bitset,
        sv.aggregate_signature.inner(),
        committee,
        &cfg,
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
    header: &ConsensusHeader,
    msg_type: ConsensusMsgType,
    vote: &Vote,
    bitset: u64,
    signature: &[u8; 48],
    committee: &Committee,
    cfg: &sortition::Config,
) -> Result<QuorumResult, Error> {
    let sub_committee = committee.intersect(bitset);

    let total = committee.total_occurrences(&sub_committee);
    let target_quorum = match vote {
        Vote::NoCandidate => committee.nil_quorum(),
        _ => committee.quorum(),
    };

    let quorum_result = QuorumResult {
        total,
        target_quorum,
    };

    if !quorum_result.quorum_reached() {
        tracing::error!(
            desc = "vote_set_too_small",
            committee = format!("{:#?}", sub_committee),
            cfg = format!("{:#?}", cfg),
            bitset,
            target_quorum,
            total,
        );
        return Err(Error::VoteSetTooSmall(cfg.step()));
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
        verify_step_signature(header, msg_type, vote, apk, signature)?;
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
    header: &ConsensusHeader,
    msg_type: ConsensusMsgType,
    vote: &Vote,
    apk: dusk_bls12_381_sign::APK,
    signature: &[u8; 48],
) -> Result<(), Error> {
    // Compile message to verify
    let sig = dusk_bls12_381_sign::Signature::from_bytes(signature)?;
    let mut msg = header.signable();
    msg.extend_from_slice(&[msg_type as u8]);
    vote.write(&mut msg).expect("Writing to vec should succeed");
    apk.verify(&sig, &msg)?;
    Ok(())
}
