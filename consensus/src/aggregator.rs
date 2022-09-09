// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{Hash, Signature};
use crate::messages::{payload, Header};
use crate::user::committee::Committee;
use crate::user::provisioners::PublicKey;
use crate::util::cluster::Cluster;
use std::collections::BTreeMap;
use std::fmt;
use tracing::{error, warn};

#[derive(Debug, Copy, Clone)]
pub struct StepVotes {
    pub bitset: u64,
    pub signature: Signature,
}

/// Aggregator collects votes per a block hash by aggregating signatures of
/// voters.StepVotes Mapping of a block hash to both an aggregated signatures
/// and a cluster of bls voters.
pub struct Aggregator(BTreeMap<Hash, (AggrSignature, Cluster<PublicKey>)>);

impl Aggregator {
    pub fn collect_vote(
        &mut self,
        committee: &Committee,
        header: Header,
        payload: payload::Reduction,
    ) -> Option<(Hash, StepVotes)> {
        let hash: Hash = header.block_hash;

        let entry = self
            .0
            .entry(hash)
            .or_insert((AggrSignature::default(), Cluster::<PublicKey>::new()));

        // Each committee has 64 slots. If a Provisioner is extracted into
        // multiple slots, then he/she only needs to send one vote which can be
        // taken account as a vote for all his/her slots. Otherwise, if a
        // Provisioner is only extracted to one slot per committee, then a single
        // vote is taken into account (if more votes for the same slot are
        // propagated, those are discarded).
        if entry.1.contains_key(&header.pubkey_bls) {
            warn!("discarding duplicated votes from a provisioner");
            return None;
        }

        // Aggregate Signatures
        if let Err(e) = entry.0.add(payload.signed_hash) {
            error!("{:?}", e);
            return None;
        }

        if let Some(weight) = committee.votes_for(header.pubkey_bls) {
            // An committee member is allowed to vote only once per a single
            // step. Its vote has a weight value depending on how many times it
            // has been extracted in the sortition for this step.
            let val = entry.1.set_weight(&header.pubkey_bls, *weight);
            debug_assert!(val != None);

            let total = entry.1.total_occurrences();
            let quorum_target = committee.quorum();
            println!("total votes: {}, quorum target: {} ", total, quorum_target);

            if total >= committee.quorum() {
                return Some((
                    hash,
                    StepVotes {
                        bitset: committee.bits(&entry.1),
                        signature: entry.0.get_aggregated(),
                    },
                ));
            }
        } else {
            error!(
                "pubkey: {} not a committee member",
                header.pubkey_bls.encode_short_hex()
            );
        }

        None
    }

    pub fn get_total(&self, hash: Hash) -> Option<usize> {
        if let Some(value) = self.0.get(&hash) {
            return Some(value.1.total_occurrences());
        }
        None
    }
}

impl Default for Aggregator {
    fn default() -> Self {
        Self(BTreeMap::default())
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
pub enum AggrSigError {}

struct AggrSignature {
    signature: Signature,
}

impl Default for AggrSignature {
    fn default() -> Self {
        Self {
            signature: Signature::default(),
        }
    }
}

impl AggrSignature {
    pub fn add(&mut self, signature: Signature) -> Result<(), AggrSigError> {
        if self.signature.is_zeroed() {
            self.signature = signature;
            return Ok(());
        }

        /* TODO: bls.AggregateSig(s.Signature, signature)
         */
        Ok(())
    }

    pub fn get_aggregated(&self) -> Signature {
        self.signature
    }
}

#[cfg(test)]
mod tests {
    use crate::aggregator::Aggregator;
    use crate::messages;
    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, PublicKey, DUSK};
    use crate::user::sortition::Config;
    use hex::FromHex;

    fn simple_pubkey(b: u8) -> PublicKey {
        let mut key: [u8; 96] = [0; 96];
        key[0] = b;

        PublicKey::new(key)
    }

    #[test]
    fn test_collect_votes() {
        let block_hash = <[u8; 32]>::from_hex(
            "b70189c7e7a347989f4fbc1205ce612f755dfc489ecf28f9f883800acf078bd5",
        )
        .unwrap();

        let empty_payload = messages::payload::Reduction {
            signed_hash: Default::default(),
        };

        // Create dummy provisioners
        let mut p = Provisioners::new();
        let mut headers = vec![];
        for i in 0..5 {
            p.add_member_with_value(simple_pubkey(i), 1000 * DUSK);

            // headers of messages voting for an empty block_hash
            headers.push(messages::Header {
                pubkey_bls: simple_pubkey(i),
                round: 0,
                step: 0,
                block_hash: block_hash,
            });
        }

        p.update_eligibility_flag(1);

        // Execute sortition with specific config
        let cfg = Config::new([0; 32], 1, 1, 64);
        let c = Committee::new(PublicKey::default(), &mut p, cfg);

        let mut a = Aggregator::default();

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
        let sv = a
            .collect_vote(&c, headers.get(2).unwrap().clone(), empty_payload)
            .unwrap();

        // Ensure returned step_votes is for the voted block hash
        assert_eq!(sv.0, block_hash);
        assert_eq!(sv.1.bitset, 6);

        println!("{:#064b}", sv.1.bitset);
        // 0b00000000000000000000000000000000000000000000000000000000000110

        // Ensure total is updated with last vote weight
        assert_eq!(a.get_total(block_hash).unwrap(), 3);
        // Ensure should reach a quorum as total is 3 >= ceil(0.67*5)
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
