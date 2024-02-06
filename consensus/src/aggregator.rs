// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::cluster::Cluster;
use crate::user::committee::Committee;
use dusk_bytes::Serializable;
use node_data::bls::PublicKey;
use node_data::ledger::{to_str, Signature, StepVotes};
use node_data::message::payload::Vote;
use node_data::message::{ConsensusHeader, SignInfo};
use std::collections::BTreeMap;
use std::fmt;
use tracing::{debug, error, warn};

/// Aggregator collects votes per a block hash by aggregating signatures of
/// voters.StepVotes Mapping of a block hash to both an aggregated signatures
/// and a cluster of bls voters.
#[derive(Default)]
pub struct Aggregator(
    BTreeMap<(u16, Vote), (AggrSignature, Cluster<PublicKey>)>,
);

impl Aggregator {
    pub fn collect_vote(
        &mut self,
        committee: &Committee,
        header: &ConsensusHeader,
        sign_info: &SignInfo,
        vote: &Vote,
        msg_step: u16,
    ) -> Option<(StepVotes, bool)> {
        let signature = sign_info.signature.inner();
        let signer = &sign_info.signer;

        // Get weight for this pubkey bls. If votes_for returns None, it means
        // the key is not a committee member, respectively we should not
        // process a vote from it.
        let weight = committee.votes_for(signer)?;

        let (aggr_sign, cluster) =
            self.0.entry((msg_step, vote.clone())).or_default();

        // Each committee has 64 slots.
        //
        // If a Provisioner is extracted into multiple slots, then he/she only
        // needs to send one vote which can be taken account as a vote for all
        // his/her slots.
        // Otherwise, if a Provisioner is only extracted to one slot per
        // committee, then a single vote is taken into account (if more votes
        // for the same slot are propagated, those are discarded).
        if cluster.contains_key(signer) {
            warn!(
                event = "discarded duplicated vote",
                from = signer.to_bs58(),
                %vote,
                msg_step,
                msg_round = header.round,
            );
            return None;
        }

        // Aggregate Signatures
        if let Err(e) = aggr_sign.add(signature) {
            error!("{:?}", e);
            return None;
        }

        // An committee member is allowed to vote only once per a single
        // step. Its vote has a weight value depending on how many times it
        // has been extracted in the sortition for this step.
        let weight = cluster.set_weight(signer, weight);
        debug_assert!(weight.is_some());

        let total = cluster.total_occurrences();
        let quorum_target = committee.super_majority_quorum();

        debug!(
            event = "vote aggregated",
            %vote,
            from = signer.to_bs58(),
            added = weight,
            total,
            target = quorum_target,
            signature = to_str(signature),
        );

        let s = aggr_sign
            .aggregated_bytes()
            .expect("Signature to exist after aggregating");
        let bitset = committee.bits(cluster);

        let step_votes = StepVotes {
            bitset,
            aggregate_signature: Signature::from(s),
        };

        let quorum_reached = match &vote {
            Vote::Valid(_) => total >= committee.super_majority_quorum(),
            _ => total >= committee.majority_quorum(),
        };

        if quorum_reached {
            tracing::info!(
                event = "quorum reached",
                %vote,
                total,
                target = quorum_target,
                bitset,
                step = msg_step,
                signature = to_str(&s),
            );
        }

        Some((step_votes, quorum_reached))
    }
}

impl fmt::Display for Aggregator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (hash, value) in self.0.iter() {
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
    data: Option<dusk_bls12_381_sign::Signature>,
}

impl AggrSignature {
    pub fn add(
        &mut self,
        data: &[u8; 48],
    ) -> Result<(), dusk_bls12_381_sign::Error> {
        let sig = BlsSignature::from_bytes(data)?;

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
    use super::*;
    use crate::aggregator::Aggregator;
    use crate::commons::RoundUpdate;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, DUSK};
    use crate::user::sortition::Config;
    use dusk_bls12_381_sign::{PublicKey, SecretKey};
    use dusk_bytes::DeserializableSlice;
    use hex::FromHex;
    use node_data::ledger::{Header, Seed};
    use node_data::message::StepMessage;
    use std::collections::HashMap;

    impl Aggregator {
        pub fn get_total(&self, step: u16, vote: Vote) -> Option<usize> {
            if let Some(value) = self.0.get(&(step, vote)) {
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
                SecretKey::from_slice(&data[..]).expect("valid secret key")
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
        let mut mrb_header = Header::default();
        mrb_header.height = 0;

        for secret_key in sks {
            let pubkey_bls =
                node_data::bls::PublicKey::new(PublicKey::from(&secret_key));

            p.add_member_with_value(pubkey_bls.clone(), 1000 * DUSK);

            let ru = RoundUpdate::new(
                pubkey_bls,
                secret_key,
                &mrb_header,
                HashMap::new(),
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
        let cfg = Config::raw(Seed::from([4u8; 48]), round, 1, 10, None);
        let c = Committee::new(&p, &cfg);

        let target_quorum = 7;

        assert_eq!(c.super_majority_quorum(), target_quorum);

        let mut a = Aggregator::default();

        dbg!("{:?}", p);

        // Collect votes from expected committee members
        let expected_members = vec![0, 1, 3, 4, 5];
        let expected_votes = vec![1, 1, 1, 2, 3];

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
        let mut collected_votes = 0;
        for i in 0..expected_members.len() - 1 {
            // Select provisioner
            let (vote, msg) =
                input.get(expected_members[i]).expect("invalid index");

            let h = msg.header();
            let sign_info = msg.sign_info();
            let step = msg.get_step();

            let vote = vote.clone();
            // Last member's vote should reach the quorum
            if i == winning_index {
                // (hash, sv) is only returned in case we reach the quorum
                let (sv, quorum_reached) = a
                    .collect_vote(&c, h, sign_info, &vote, step)
                    .expect("failed to reach quorum");

                assert!(quorum_reached, "quorum should be reached");

                assert_eq!(vote, init_vote);

                // Check expected StepVotes bitset
                // bitset: 0b00000000000000000000000000000000000000000000000000000000011111
                println!("bitset: {:#064b}", sv.bitset);
                assert_eq!(sv.bitset, 31);

                break;
            }

            // Check collected votes
            let (_, quorum_reached) =
                a.collect_vote(&c, h, sign_info, &vote, step).unwrap();

            assert!(!quorum_reached, "quorum should not be reached yet");

            collected_votes += expected_votes[i];
            assert_eq!(a.get_total(step, vote.clone()), Some(collected_votes));

            // Ensure a duplicated vote is discarded
            if i == 0 {
                assert!(a
                    .collect_vote(&c, h, sign_info, &vote, step)
                    .is_none());
            }
        }
    }
}
