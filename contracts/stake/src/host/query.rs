// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, Counter, Stake};
use canonical_host::{MemStore, Query};
use dusk_bls12_381_sign::APK;

type QueryIndex = u16;

impl Contract<MemStore> {
    pub fn find_stake(
        w_i: Counter,
        pk: APK,
    ) -> Query<(QueryIndex, Counter, APK), Option<Stake>> {
        Query::new((ops::FIND_STAKE, w_i, pk))
    }
}
