// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Contract;
use canonical::Store;
use dusk_bls12_381::BlsScalar;

impl<S: Store> Contract<S> {
    pub fn find_bid(&self, idx: u64) -> () {
        unimplemented!()
    }
}
