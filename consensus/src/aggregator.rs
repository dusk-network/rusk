// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Hash;
use crate::messages::payload::StepVotes;
use crate::messages::{payload, Header};
use crate::user::committee::Committee;
use crate::util::cluster::Cluster;
use crate::util::pubkey::ConsensusPublicKey;
use dusk_bytes::Serializable;
use std::collections::BTreeMap;
use std::fmt;
use tracing::{error, warn};

/// Aggregator collects votes per a block hash by aggregating signatures of
/// voters.StepVotes Mapping of a block hash to both an aggregated signatures
/// and a cluster of bls voters.
#[derive(Default)]
pub struct Aggregator(BTreeMap<Hash, (AggrSignature, Cluster<ConsensusPublicKey>)>);

impl Aggregator {
    pub fn collect_vote(
        &mut self,
        committee: &Committee,
        header: Header,
        payload: payload::Reduction,
    ) -> Option<(Hash, StepVotes)> {
        // Get weight for this pubkey bls. If it is 0, it means the key is not a committee member,
        // respectively we should not process a vote from it.
        if let Some(weight) = committee.votes_for(header.pubkey_bls) {
            let hash: Hash = header.block_hash;

            let (aggr_sign, cluster) = self
                .0
                .entry(hash)
                .or_insert((AggrSignature::default(), Cluster::new()));

            // Each committee has 64 slots. If a Provisioner is extracted into
            // multiple slots, then he/she only needs to send one vote which can be
            // taken account as a vote for all his/her slots. Otherwise, if a
            // Provisioner is only extracted to one slot per committee, then a single
            // vote is taken into account (if more votes for the same slot are
            // propagated, those are discarded).
            if cluster.contains_key(&header.pubkey_bls) {
                warn!("discarding duplicated votes from a provisioner");
                return None;
            }

            // Aggregate Signatures
            if let Err(e) = aggr_sign.add(payload.signed_hash) {
                error!("{:?}", e);
                return None;
            }

            // An committee member is allowed to vote only once per a single
            // step. Its vote has a weight value depending on how many times it
            // has been extracted in the sortition for this step.
            let val = cluster.set_weight(&header.pubkey_bls, *weight);
            debug_assert!(val != None);

            let total = cluster.total_occurrences();
            let quorum_target = committee.quorum();
            tracing::trace!("total votes: {}, quorum target: {} ", total, quorum_target);

            if total >= committee.quorum() {
                let signature = aggr_sign
                    .aggregated_bytes()
                    .expect("Signature to exist after quorum reached");
                let bitset = committee.bits(cluster);

                let step_votes = StepVotes { bitset, signature };

                return Some((hash, step_votes));
            }
        }

        None
    }
}

impl fmt::Display for Aggregator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (hash, value) in self.0.iter() {
            writeln!(f, "hash: {:?} total: {}", hash, value.1.total_occurrences())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum AggrSigError {
    InvalidData(dusk_bls12_381_sign::Error),
}

impl From<dusk_bls12_381_sign::Error> for AggrSigError {
    fn from(e: dusk_bls12_381_sign::Error) -> Self {
        Self::InvalidData(e)
    }
}

#[derive(Default)]
struct AggrSignature {
    data: Option<dusk_bls12_381_sign::Signature>,
}

impl AggrSignature {
    pub fn add(&mut self, data: [u8; 48]) -> Result<(), AggrSigError> {
        let sig = dusk_bls12_381_sign::Signature::from_bytes(&data)?;

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

/* TODO: Enable aggregator unit test with hard-coded seeds for both golang and rustlang implementations
#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregator::Aggregator;
    use crate::messages;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, DUSK};
    use crate::user::sortition::Config;
    use dusk_bls12_381_sign::PublicKey;
    use hex::FromHex;
    impl Aggregator {
        pub fn get_total(&self, hash: Hash) -> Option<usize> {
            if let Some(value) = self.0.get(&hash) {
                return Some(value.1.total_occurrences());
            }
            None
        }
    }

    fn simple_pubkey(b: u8) -> ConsensusPublicKey {
        let mut key: [u8; 96] = [0; 96];
        key[0] = b;

        unsafe { ConsensusPublicKey::new(PublicKey::from_slice_unchecked(&key)) }
    }

    #[test]
    fn test_collect_votes() {
        let round = 1;
        let step = 1;

        let block_hash = <[u8; 32]>::from_hex(
            "b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5",
        )
        .unwrap();

        let empty_payload = messages::payload::Reduction {
            signed_hash: [0; 48],
        };

        // Create dummy provisioners
        let mut p = Provisioners::new();
        let mut headers = vec![];
        for i in 0..5 {
            p.add_member_with_value(simple_pubkey(i), 1000 * DUSK);

            // headers of messages voting for an empty block_hash
            headers.push(messages::Header {
                pubkey_bls: simple_pubkey(i),
                round,
                step,
                block_hash,
            });
        }

        p.update_eligibility_flag(1);

        // Execute sortition with specific config
        let cfg = Config::new([0; 32], round, step, 64);
        let c = Committee::new(ConsensusPublicKey::new(PublicKey::default()), &mut p, cfg);

        assert_eq!(c.quorum(), 4);

        let mut a = Aggregator::default();

        // Ensure voting with a non-committee member, no change is applied.
        assert_eq!(
            a.collect_vote(&c, headers.get(0).unwrap().clone(), empty_payload)
                .is_none(),
            true
        );

        assert_eq!(a.get_total(block_hash).is_none(), true);

        // Ensure voting with a committee member, total_occurrences for this hash is updated.
        assert_eq!(
            a.collect_vote(&c, headers.get(1).unwrap().clone(), empty_payload)
                .is_none(),
            true
        );

        // this provisioner has weight of 2. A single vote from it should increase the total to 2
        assert_eq!(a.get_total(block_hash).unwrap(), 2);

        // Same bls key sending a vote, should be rejected
        assert_eq!(
            a.collect_vote(&c, headers.get(1).unwrap().clone(), empty_payload)
                .is_none(),
            true
        );

        // total unchanged after a duplicated vote
        assert_eq!(a.get_total(block_hash).unwrap(), 2);

        // Simulate another provisioner sending a vote. This one has a weight of 1.
        assert_eq!(
            a.collect_vote(&c, headers.get(2).unwrap().clone(), empty_payload)
                .is_none(),
            true
        );

        // Ensure total is updated with last vote weight
        assert_eq!(a.get_total(block_hash).unwrap(), 3);

        let sv = a
            .collect_vote(&c, headers.get(3).unwrap().clone(), empty_payload)
            .unwrap();

        // Ensure returned step_votes is for the voted block hash
        assert_eq!(sv.0, block_hash);

        // println!("bitset: {:#064b}", sv.1.bitset);
        // bitset: 0b00000000000000000000000000000000000000000000000000000000000111

        assert_eq!(sv.1.bitset, 7);

        // Ensure should reach a quorum as total is 4 >= ceil(0.67*5)
        assert!(a.get_total(block_hash).unwrap() >= c.quorum());

        // msg header voting for an empty block hash
        let header_with_empty_block_hash = messages::Header {
            pubkey_bls: simple_pubkey(10),
            round: 0,
            step: 0,
            block_hash: [0; 32],
        };

        // Vote for an empty block hash.
        // Ensure this returns None as we don't have enough votes for an empty_block_hash yet.
        assert_eq!(
            a.collect_vote(&c, header_with_empty_block_hash, empty_payload)
                .is_none(),
            true
        );
    }
}


 */
