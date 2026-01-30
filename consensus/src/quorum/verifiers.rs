// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable as BytesSerializable;
use dusk_core::signatures::bls::{
    MultisigPublicKey as BlsMultisigPublicKey,
    MultisigSignature as BlsMultisigSignature,
};
use node_data::bls::PublicKey;
use node_data::ledger::{Seed, ShortHex, StepVotes};
use node_data::message::payload::{self, Vote};
use node_data::message::{ConsensusHeader, SignedStepMessage};
use node_data::{Serializable, StepName};
use tokio::sync::RwLock;
use tracing::error;

use crate::config::exclude_next_generator;
use crate::errors::StepSigError;
use crate::operations::Voter;
use crate::user::cluster::Cluster;
use crate::user::committee::{Committee, CommitteeSet};
use crate::user::sortition;

pub async fn verify_step_votes(
    ch: &ConsensusHeader,
    vote: &Vote,
    step_votes: &StepVotes,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    step: StepName,
) -> Result<Vec<Voter>, StepSigError> {
    // When verifying a NoQuorum Attestation, the Validation StepVotes should be
    // empty. To be on the safe side, we simply skip verification, instead of
    // failing verification
    if step == StepName::Validation && *vote == Vote::NoQuorum {
        return Ok(vec![]);
    }

    let committee = get_step_committee(ch, committees_set, seed, step).await;

    // Verify the aggregated signature is valid and reach the quorum threshold
    let voters = verify_quorum_votes(ch, step, vote, step_votes, &committee)
        .inspect_err(|e| {
            error!(
                event = "Invalid StepVotes",
                reason = %e,
                ?vote,
                round = ch.round,
                iter = ch.iteration,
                ?step,
                seed = seed.inner().hex(),
                ?step_votes
            );
        })?;

    Ok(voters)
}

pub fn verify_quorum_votes(
    header: &ConsensusHeader,
    step: StepName,
    vote: &Vote,
    step_votes: &StepVotes,
    committee: &Committee,
) -> Result<Vec<Voter>, StepSigError> {
    let bitset = step_votes.bitset;
    let signature = step_votes.aggregate_signature().inner();
    let sub_committee = committee.intersect(bitset);

    let total_credits = committee.total_occurrences(&sub_committee);
    let quorum_threshold = match vote {
        Vote::Valid(_) => committee.super_majority_quorum(),
        _ => committee.majority_quorum(),
    };

    // Check credits reach the quorum
    if total_credits < quorum_threshold {
        error!(
            event = "Invalid quorum",
            reason = "Credits below the quorum threhsold",
            total_credits,
            quorum_threshold,
            committee = format!("{committee}"),
            sub_committee = format!("{:#?}", sub_committee),
            bitset,
            ?vote
        );
        return Err(StepSigError::VoteSetTooSmall);
    }

    // Verify aggregated signature
    let apk = sub_committee.aggregate_pks()?;
    verify_step_signature(header, step, vote, apk, signature)?;

    Ok(sub_committee.to_voters())
}

impl Cluster<PublicKey> {
    fn aggregate_pks(&self) -> Result<BlsMultisigPublicKey, StepSigError> {
        let pks: Vec<_> =
            self.iter().map(|(pubkey, _)| *pubkey.inner()).collect();
        Ok(BlsMultisigPublicKey::aggregate(&pks)?)
    }

    pub fn to_voters(self) -> Vec<Voter> {
        self.into_vec()
    }
}

fn verify_step_signature(
    header: &ConsensusHeader,
    step: StepName,
    vote: &Vote,
    apk: BlsMultisigPublicKey,
    signature: &[u8; 48],
) -> Result<(), StepSigError> {
    // Compile message to verify
    let sign_seed = match step {
        StepName::Validation => payload::Validation::SIGN_SEED,
        StepName::Ratification => payload::Ratification::SIGN_SEED,
        StepName::Proposal => Err(StepSigError::InvalidType)?,
    };

    let sig = BlsMultisigSignature::from_bytes(signature)?;
    let mut msg = header.signable();
    msg.extend_from_slice(sign_seed);
    vote.write(&mut msg).expect("Writing to vec should succeed");
    apk.verify(&sig, &msg)?;
    Ok(())
}

pub async fn get_step_voters(
    header: &ConsensusHeader,
    step_votes: &StepVotes,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    step: StepName,
) -> Vec<Voter> {
    // compute committee for `step`
    let committee =
        get_step_committee(header, committees_set, seed, step).await;

    // extract quorum voters from `step_votes`
    let bitset = step_votes.bitset;
    let q_committee = committee.intersect(bitset);

    q_committee.to_voters()
}

async fn get_step_committee(
    header: &ConsensusHeader,
    committees_set: &RwLock<CommitteeSet<'_>>,
    seed: Seed,
    step: StepName,
) -> Committee {
    let round = header.round;
    let iteration = header.iteration;

    // exclude current-iteration generator
    let mut exclusion_list = vec![];
    let generator = committees_set
        .read()
        .await
        .provisioners()
        .get_generator(iteration, seed, round);

    exclusion_list.push(generator);

    // exclude next-iteration generator
    if exclude_next_generator(iteration) {
        let next_generator = committees_set
            .read()
            .await
            .provisioners()
            .get_generator(iteration + 1, seed, round);

        exclusion_list.push(next_generator);
    }

    let cfg =
        sortition::Config::new(seed, round, iteration, step, exclusion_list);

    if committees_set.read().await.get(&cfg).is_none() {
        let _ = committees_set.write().await.get_or_create(&cfg);
    }

    let set = committees_set.read().await;
    let committee = set.get(&cfg).expect("committee to be created");

    committee.clone()
}
