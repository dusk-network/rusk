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
use tracing::{debug, error, warn};

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
            panic!("{:?}", e);
        }

        if let Some(weight) = committee.votes_for(header.pubkey_bls) {
            // An eligible provisioner is allowed to vote only once per a single
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

mod tests {
    use crate::aggregator::Aggregator;
    use crate::commons::Hash;
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
                block_hash: [0; 32],
            });
        }

        p.update_eligibility_flag(1);

        // Execute sortition with specific config
        let cfg = Config::new([0; 32], 1, 1, 64);
        let c = Committee::new(PublicKey::default(), &mut p, cfg);
        let mut a = Aggregator::default();

        let payload = messages::payload::Reduction {
            signed_hash: Default::default(),
        };

        assert_eq!(
            a.collect_vote(&c, headers.get(1).unwrap().clone(), payload.clone())
                .is_none(),
            true
        );

        // this provisioner has weight of 2. A single vote from it, should increase the total to 2
        assert_eq!(a.get_total([0; 32]).unwrap(), 2);

        // Same bls key sending a vote, should be rejected
        assert_eq!(
            a.collect_vote(&c, headers.get(1).unwrap().clone(), payload.clone())
                .is_none(),
            true
        );

        // total unchanged after a duplicated vote
        assert_eq!(a.get_total([0; 32]).unwrap(), 2);

        // Simulate another provisioner is sending a vote. This one has a weight of 1.

        assert_eq!(
            a.collect_vote(&c, headers.get(2).unwrap().clone(), payload.clone())
                .is_none(),
            false
        );

        // total is updated with last vote weight
        // Now we should reach a quorum as total is 3 >= ceil(0.67*5)
        assert_eq!(a.get_total([0; 32]).unwrap(), 3);
        assert!(a.get_total([0; 32]).unwrap() >= c.quorum());
    }
}
