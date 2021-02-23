// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::vec::Vec;
use dusk_bls12_381::BlsScalar;

use canonical::{BridgeStore, Id32};
use dusk_abi::Module;

type BS = BridgeStore<Id32>;
type RuskModule = crate::RuskModule<BS>;

pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    dusk_abi::query(&RuskModule::id(), &(RuskModule::POSEIDON_HASH, scalars))
        .expect("query ZK Module for Poseidon Hash should not fail")
}
