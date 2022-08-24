// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::provisioners::{HashablePubKey, Provisioners};
use dusk_bls12_381_sign::PublicKey;
use std::collections::HashMap;
use tracing::trace;

#[allow(unused)]
// 0: BLS Public key mapped to its occurrences.
// 1: This provisioner BLS public key.
#[derive(Default, Debug)]
pub struct Committee(HashMap<HashablePubKey, usize>, PublicKey);

#[allow(unused)]
impl Committee {
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &mut Provisioners,
        seed: [u8; 32],
        round: u64,
        step: u8,
        size: usize,
    ) -> Self {
        let mut committee = Self::default();

        // Generate committee using deterministic sortition.
        let res = provisioners.create_committee(seed, round, step, size);

        // Turn the raw vector into a hashmap where we map a pubkey to its occurrences.
        for member_key in res.as_slice() {
            *committee
                .0
                .entry(HashablePubKey::new(member_key.clone()))
                .or_insert(0) += 1;
        }

        trace!(
            "committee at round/step {}/{} {:?}",
            round,
            step,
            &committee
        );

        committee
    }

    pub fn is_member(&self, pubkey_bls: PublicKey) -> bool {
        self.0.contains_key(&HashablePubKey::new(pubkey_bls))
    }

    pub fn am_member(&self) -> bool {
        self.is_member(self.1)
    }

    pub fn votes_for(&self, pubkey_bls: PublicKey) -> Option<&usize> {
        self.0.get(&HashablePubKey::new(pubkey_bls))
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }
}
