// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, PublicKeys};
use canonical_host::{MemStore, Transaction};
use dusk_bls12_381_sign::{Signature, APK};

type TransactionIndex = u16;

impl Contract<MemStore> {
    pub fn distribute(
        value: u64,
        public_keys: PublicKeys<MemStore>,
    ) -> Transaction<(TransactionIndex, u64, PublicKeys<MemStore>), bool> {
        Transaction::new((ops::DISTRIBUTE, value, public_keys))
    }

    pub fn withdraw(
        block_height: u64,
        public_key: APK,
        sig: Signature,
        /* note */
    ) -> Transaction<(TransactionIndex, u64, APK, Signature), bool> {
        Transaction::new((ops::WITHDRAW, block_height, public_key, sig))
    }
}
