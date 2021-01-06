// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract};
use canonical_host::{MemStore, Query};
use dusk_bls12_381_sign::APK;

type QueryIndex = u16;

impl Contract<MemStore> {
    pub fn get_balance(pk: APK) -> Query<(QueryIndex, APK), Option<u64>> {
        Query::new((ops::GET_BALANCE, pk))
    }

    pub fn get_withdrawal_time(
        pk: APK,
    ) -> Query<(QueryIndex, APK), Option<u64>> {
        Query::new((ops::GET_WITHDRAWAL_TIME, pk))
    }
}
