// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::consensus::CONSENSUS_QUORUM_THRESHOLD;
use crate::user::provisioners::{Provisioners, PublicKey};
use crate::user::sortition;
use crate::util::cluster::Cluster;
use math::round;
use std::collections::BTreeMap;
use std::mem;
use tracing::error;

#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct Committee {
    members: BTreeMap<PublicKey, usize>,
    this_member_key: PublicKey,
    cfg: sortition::Config,
    total: usize,
}

#[allow(unused)]
impl Committee {
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &mut Provisioners,
        cfg: sortition::Config,
    ) -> Self {
        // Generate committee using deterministic sortition.
        let res = provisioners.create_committee(&cfg);

        // Turn the raw vector into a hashmap where we map a pubkey to its occurrences.
        let mut committee = Self {
            members: BTreeMap::new(),
            this_member_key: pubkey_bls,
            cfg: cfg.clone(),
            total: 0,
        };

        for member_key in res.as_slice() {
            *committee.members.entry(*member_key).or_insert(0) += 1;
            committee.total += 1;
        }

        committee
    }

    pub fn is_member(&self, pubkey_bls: PublicKey) -> bool {
        self.members.contains_key(&pubkey_bls)
    }

    pub fn am_member(&self) -> bool {
        self.is_member(self.this_member_key)
    }

    // get_my_pubkey returns this provisioner BLS public key.
    pub fn get_my_pubkey(&self) -> PublicKey {
        self.this_member_key
    }

    pub fn votes_for(&self, pubkey_bls: PublicKey) -> Option<&usize> {
        self.members.get(&pubkey_bls)
    }

    // get_occurrences returns values in a vec
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.members.clone().into_values().collect()
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn quorum(&self) -> usize {
        let size = self.total as f64;
        round::ceil(size * CONSENSUS_QUORUM_THRESHOLD, 2) as usize
    }

    pub fn bits(&self, voters: &Cluster<PublicKey>) -> u64 {
        let mut bits: u64 = 0;

        debug_assert!(self.members.len() <= mem::size_of_val(&bits) * 8);

        let mut pos = 0;
        for item in voters.0.iter() {
            pos = 0;
            for member in self.members.iter() {
                pos += 1;
                if member.0 == item.0 {
                    bits |= 1 << pos; // flip the i-th bit to 1
                    break;
                }
            }
        }

        bits
    }
}
