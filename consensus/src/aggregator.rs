// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use dusk_bytes::Serializable;
use execution_core::signatures::bls::{
    Error as BlsSigError, MultisigSignature as BlsMultisigSignature,
};
use node_data::bls::{PublicKey, PublicKeyBytes};
use node_data::ledger::{to_str, StepVotes};
use node_data::message::payload::Vote;
use node_data::message::SignedStepMessage;
use thiserror::Error;
use tracing::{debug, error, warn};

use crate::config::is_emergency_iter;
use crate::user::cluster::Cluster;
use crate::user::committee::Committee;

/// Aggregator collects votes for Validation and Ratification steps by
/// mapping step numbers and [StepVote] to both an aggregated signature and a
/// cluster of voters.
///
/// It ensures that no multiple votes for same voter are collected.
pub struct Aggregator<V> {
    // Map between (step, vote) and (signature, voters)
    votes: BTreeMap<(u8, Vote), (AggrSignature, Cluster<PublicKey>)>,

    // Map each step to the set of voters. We do this to ensure only one vote
    // per voter is cast
    uniqueness: BTreeMap<u8, HashMap<PublicKeyBytes, V>>,
}

impl<V> Default for Aggregator<V> {
    fn default() -> Self {
        Self {
            votes: BTreeMap::default(),
            uniqueness: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum AggregatorError<V> {
    #[error("Vote already aggregated")]
    DuplicatedVote,
    #[error("Vote conflicted with previous one")]
    ConflictingVote(V),
    #[error("Vote from member not in the committee")]
    NotCommitteeMember,
    #[error("Invalid signature to aggregate {0}")]
    InvalidSignature(BlsSigError),
}

impl<V> From<BlsSigError> for AggregatorError<V> {
    fn from(value: BlsSigError) -> Self {
        Self::InvalidSignature(value)
    }
}

pub trait StepVote: Clone + SignedStepMessage {
    fn vote(&self) -> &Vote;
}

impl<V: StepVote> Aggregator<V> {
    pub fn is_vote_collected(&self, v: &V) -> bool {
        let signer = &v.sign_info().signer;
        let msg_step = v.get_step();
        let vote = v.vote();

        self.votes
            .get(&(msg_step, *vote))
            .map_or(false, |(_, cluster)| cluster.contains_key(signer))
    }

    pub fn collect_vote(
        &mut self,
        committee: &Committee,
        v: &V,
    ) -> Result<(StepVotes, bool), AggregatorError<V>> {
        let sign_info = v.sign_info();

        let iter = v.header().iteration;

        let emergency = is_emergency_iter(iter);

        let msg_step = v.get_step();
        let vote = v.vote();
        if emergency && !vote.is_valid() {
            warn!(
                "Vote {vote:?} for iter {iter} skipped due to emergency mode",
            );
            return Ok((StepVotes::default(), false));
        }

        let signature = sign_info.signature.inner();
        let signer = &sign_info.signer;

        // Get weight for this pubkey bls. If votes_for returns None, it means
        // the key is not a committee member, respectively we should not
        // process a vote from it.
        let weight = committee
            .votes_for(signer)
            .ok_or(AggregatorError::NotCommitteeMember)?;

        let (aggr_sign, cluster) =
            self.votes.entry((msg_step, *vote)).or_default();

        // Each committee has 64 slots.
        //
        // If a Provisioner is extracted into multiple slots, then he/she only
        // needs to send one vote which can be taken account as a vote for all
        // his/her slots.
        // Otherwise, if a Provisioner is only extracted to one slot per
        // committee, then a single vote is taken into account (if more votes
        // for the same slot are propagated, those are discarded).
        if cluster.contains_key(signer) {
            return Err(AggregatorError::DuplicatedVote);
        }

        if !emergency {
            // Check if the provisioner voted for a different result
            let voters_list = self.uniqueness.entry(msg_step).or_default();
            match voters_list.get(signer.bytes()) {
                None => voters_list.insert(*signer.bytes(), v.clone()),
                Some(prev_vote) => {
                    return Err(AggregatorError::ConflictingVote(
                        prev_vote.clone(),
                    ))
                }
            };
        }

        // Aggregate Signatures
        aggr_sign.add(signature)?;

        // An committee member is allowed to vote only once per a single
        // step. Its vote has a weight value depending on how many times it
        // has been extracted in the sortition for this step.
        let added = cluster
            .add(signer, weight)
            .expect("Vote to be added to cluster");

        let total = cluster.total_occurrences();

        debug!(
            event = "vote aggregated",
            ?vote,
            from = signer.to_bs58(),
            iter = v.header().iteration,
            step = ?V::STEP_NAME,
            added,
            total,
            majority = committee.majority_quorum(),
            super_majority = committee.super_majority_quorum(),
            signature = to_str(signature),
        );

        let aggregate_signature = aggr_sign
            .aggregated_bytes()
            .expect("Signature to exist after aggregating");
        let bitset = committee.bits(cluster);

        let step_votes = StepVotes::new(aggregate_signature, bitset);

        let quorum_target = match &vote {
            Vote::Valid(_) => committee.super_majority_quorum(),
            _ => committee.majority_quorum(),
        };

        let quorum_reached = total >= quorum_target;
        if quorum_reached {
            tracing::info!(
                event = "quorum reached",
                ?vote,
                iter = v.header().iteration,
                step = ?V::STEP_NAME,
                total,
                target = quorum_target,
                bitset,
                step = msg_step,
                signature = to_str(&aggregate_signature),
            );
        }

        Ok((step_votes, quorum_reached))
    }
}

impl<V> fmt::Display for Aggregator<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (hash, value) in self.votes.iter() {
            writeln!(
                f,
                "hash: {:?} total: {}",
                hash,
                value.1.total_occurrences()
            )?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct AggrSignature {
    data: Option<BlsMultisigSignature>,
}

impl AggrSignature {
    pub fn add(&mut self, data: &[u8; 48]) -> Result<(), BlsSigError> {
        let sig = BlsMultisigSignature::from_bytes(data)?;

        let aggr_sig = match self.data {
            Some(data) => data.aggregate(&[sig]),
            None => sig,
        };

        self.data = Some(aggr_sig);

        Ok(())
    }

    pub fn aggregated_bytes(&self) -> Option<[u8; 48]> {
        self.data.map(|sig| sig.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use dusk_bytes::DeserializableSlice;
    use execution_core::signatures::bls::{
        PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    };
    use hex::FromHex;
    use node_data::ledger::{Header, Seed};
    use node_data::message::StepMessage;

    use super::*;
    use crate::aggregator::Aggregator;
    use crate::commons::RoundUpdate;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, DUSK};
    use crate::user::sortition::Config;

    impl<V> Aggregator<V> {
        pub fn get_total(&self, step: u8, vote: Vote) -> Option<usize> {
            if let Some(value) = self.votes.get(&(step, vote)) {
                return Some(value.1.total_occurrences());
            }
            None
        }
    }

    #[test]
    fn test_collect_votes() {
        let sks = [
            "7f6f2ccdb23f2abb7b69278e947c01c6160a31cf02c19d06d0f6e5ab1d768b15",
            "611830d3641a68f94a690dcc25d1f4b0dac948325ac18f6dd32564371735f32c",
            "1fbec814b18b1d4c3eaa7cec41007e04bf0a98453b06ec7582aa29882c52eb3e",
            "ecd9c4a53ea15f18447b08fb96a13c5ab7dc7d24067b102fcbaaf7b39ca52e2d",
            "e463bcb1a6e57288ffd4671503082fa8656e3eacb78fb1925f8a7c76400e8e15",
            "7a19fb2d099a9557f7c10c2efbb8b101d9e0ec85610d5c74a887d1d4fb8d2827",
            "4dbad51eb408af559dd91bbbed8dbeae0a2c89e0e05f0cce87c98652a8437f1f",
            "befba86ae9e0c207865f7e24e8349d4ecdbc8b0f4632842499a0dfa60568e20a",
            "b260b8a10343bf5a5dacb4f1d32d06c4fdddc9981a3619fbc0a5cd9eb30f3334",
            "87a9779748888da5d96bbbce041b5109c6ffc0c4f30561c0170384a5922d9e21",
        ];
        let sks: Vec<_> = sks
            .iter()
            .map(|hex| hex::decode(hex).expect("valid hex"))
            .map(|data| {
                BlsSecretKey::from_slice(&data[..]).expect("valid secret key")
            })
            .collect();

        let round = 1;
        let iteration = 1;

        let block_hash = <[u8; 32]>::from_hex(
            "b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5",
        )
        .unwrap();
        let init_vote = Vote::Valid(block_hash);

        // Create provisioners
        // Also populate a vector of headers
        let mut p = Provisioners::empty();
        let mut input = vec![];
        let mut tip_header = Header::default();
        tip_header.height = 0;

        for secret_key in sks {
            let pubkey_bls =
                node_data::bls::PublicKey::new(BlsPublicKey::from(&secret_key));

            p.add_member_with_value(pubkey_bls.clone(), 1000 * DUSK);

            let ru = RoundUpdate::new(
                pubkey_bls,
                secret_key,
                &tip_header,
                HashMap::new(),
                vec![],
            );

            let msg = crate::build_validation_payload(
                init_vote.clone(),
                &ru,
                iteration,
            );

            // Message headers to be used in test for voting for hash:
            // block_hash
            input.push((msg.vote.clone(), msg));
        }

        // Execute sortition with specific config
        let cfg = Config::raw(Seed::from([4u8; 48]), round, 1, 10, vec![]);
        let c = Committee::new(&p, &cfg);

        let target_quorum = 7;

        assert_eq!(c.super_majority_quorum(), target_quorum);

        let mut a = Aggregator::default();

        dbg!("{:?}", p);

        // Collect votes from expected committee members
        let expected_members = vec![1, 2, 3, 4];
        let expected_votes = vec![1, 1, 2, 1];

        // The index of the provisioner (inside expected_members) that let the
        // quorum being reached
        let (winning_index, _) = expected_votes.iter().enumerate().fold(
            (0, 0),
            |(index, current_quorum), (i, &value)| {
                if current_quorum >= target_quorum {
                    (index, current_quorum)
                } else {
                    (i, current_quorum + value)
                }
            },
        );
        println!("winning index {winning_index}");
        let mut collected_votes = 0;
        for i in 0..expected_members.len() - 1 {
            // Select provisioner
            let (vote, msg) =
                input.get(expected_members[i]).expect("invalid index");

            let vote = vote.clone();
            // Last member's vote should reach the quorum
            if i == winning_index {
                // (hash, sv) is only returned in case we reach the quorum
                let (sv, quorum_reached) =
                    a.collect_vote(&c, msg).expect("failed to reach quorum");

                assert!(quorum_reached, "quorum should be reached");

                assert_eq!(vote, init_vote);

                // Check expected StepVotes bitset
                // bitset: 0b00000000000000000000000000000000000000000000000000000000011111
                println!("bitset: {:#064b}", sv.bitset);
                assert_eq!(sv.bitset, 31);

                break;
            }

            println!("Collecting vote for index {i}");
            // Check collected votes
            let (_, quorum_reached) = a.collect_vote(&c, msg).unwrap();

            assert!(!quorum_reached, "quorum should not be reached yet");

            collected_votes += expected_votes[i];
            assert_eq!(
                a.get_total(msg.get_step(), msg.vote),
                Some(collected_votes)
            );

            if i == 0 {
                // Ensure a duplicated vote is discarded
                match a.collect_vote(&c, msg) {
                    Err(AggregatorError::DuplicatedVote) => {}
                    _ => panic!("Vote should be discarded"),
                }

                // Ensure a conflicting vote is discarded
                let mut wrong_msg = msg.clone();
                wrong_msg.vote = Vote::Invalid(block_hash);
                match a.collect_vote(&c, &wrong_msg) {
                    Err(AggregatorError::ConflictingVote(m)) => {
                        assert_eq!(&m, msg)
                    }
                    _ => panic!("Vote should be discarded as conflicting"),
                }
            }
        }
    }
}
