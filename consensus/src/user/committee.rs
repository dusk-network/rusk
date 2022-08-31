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
#[derive(Default, Debug, Clone)]
pub struct Committee(BTreeMap<PublicKey, usize>, PublicKey);

#[allow(unused)]
impl Committee {
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &mut Provisioners,
        cfg: sortition::Config,
    ) -> Self {
        // Generate committee using deterministic sortition.
        let res = provisioners.create_committee(cfg);

        // Turn the raw vector into a hashmap where we map a pubkey to its occurrences.
        let mut committee = Self(BTreeMap::new(), pubkey_bls);
        for member_key in res.as_slice() {
            *committee.0.entry(*member_key).or_insert(0) += 1;
        }

        committee
    }

    pub fn is_member(&self, pubkey_bls: PublicKey) -> bool {
        self.0.contains_key(&pubkey_bls)
    }

    pub fn am_member(&self) -> bool {
        self.is_member(self.1)
    }

    // get_my_pubkey returns this provisioner BLS public key.
    pub fn get_my_pubkey(&self) -> PublicKey {
        self.1
    }

    pub fn votes_for(&self, pubkey_bls: PublicKey) -> Option<&usize> {
        self.0.get(&pubkey_bls)
    }

    // get_occurrences returns values in a vec
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.0.clone().into_values().collect()
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }
}
