// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Contract;

use canonical::Store;
use dusk_bls12_381::BlsScalar;

impl<S: Store> Contract<S> {
    pub fn get_balance(&self, address: BlsScalar) -> u64 {
        self.balance()
            .get(&address)
            .map(|v| v.unwrap_or_default())
            .unwrap_or_default()
    }
}
