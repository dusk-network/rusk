// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Tree;

use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_kelvin_map::Map;

#[derive(Debug, Default, Clone, Canon)]
pub struct Contract<S: Store> {
    notes: Tree<S>,
    nullifiers: Map<BlsScalar, (), S>,
    roots: Map<BlsScalar, (), S>,
    balance: Map<BlsScalar, u64, S>,
}

impl<S: Store> Contract<S> {
    // TODO convert to const fn
    // https://github.com/rust-lang/rust/issues/57563
    pub fn minimum_gas_price() -> u64 {
        // TODO define the mininum gas price
        0
    }

    pub fn any_nullifier_exists(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<bool, S::Error> {
        nullifiers.iter().try_fold(false, |t, n| {
            Ok(t || self.nullifiers.get(n).map(|n| n.is_some())?)
        })
    }

    pub fn root_exists(&self, root: &BlsScalar) -> Result<bool, S::Error> {
        self.roots.get(root).map(|t| t.is_some())
    }

    pub fn balance(&self) -> &Map<BlsScalar, u64, S> {
        &self.balance
    }

    pub fn balance_mut(&mut self) -> &mut Map<BlsScalar, u64, S> {
        &mut self.balance
    }
}
