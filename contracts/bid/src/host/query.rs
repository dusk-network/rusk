// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Contract;
use canonical_host::{MemStore, Query};
use dusk_blindbid::bid::Bid;

type QueryIndex = u16;

impl Contract<MemStore> {
    pub fn find_bid() -> Query<(QueryIndex, u64), [Bid; 2]> {
        //Query::new((ops::GET_LEAF, pos as u64))
        unimplemented!()
    }
}
