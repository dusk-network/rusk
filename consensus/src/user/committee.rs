// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::provisioners::{Provisioners, PublicKey};
use crate::user::sortition;
use std::collections::BTreeMap;

#[allow(unused)]
// 0: BLS public key mapped to its occurrences.
// 1: This provisioner BLS public key.
#[derive(Default, Debug)]
pub struct Committee(BTreeMap<PublicKey, usize>, PublicKey);

#[allow(unused)]
impl Committee {
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &mut Provisioners,
        cfg: sortition::Config,
    ) -> Self {
        // Generate committee using deterministic sortition.
        // TODO: Provisioners list in golang impl is sorted by big.Int representation of a BLS key.
        //
        let res = provisioners.create_committee(cfg.clone());

        // Turn the raw vector into a hashmap where we map a pubkey to its occurrences.
        let mut committee = Self {
            0: BTreeMap::new(),
            1: pubkey_bls,
        };

        for member_key in res.as_slice() {
            *committee.0.entry(member_key.clone()).or_insert(0) += 1;
        }

        committee
    }

    pub fn is_member(&self, pubkey_bls: PublicKey) -> bool {
        self.0.contains_key(&pubkey_bls)
    }

    pub fn am_member(&self) -> bool {
        self.is_member(self.1)
    }

    pub fn votes_for(&self, pubkey_bls: PublicKey) -> Option<&usize> {
        self.0.get(&pubkey_bls)
    }

    // get_occurrences returns values in a sorted vec. (testing purposes only)
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.0.clone().into_values().collect()
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }
}
