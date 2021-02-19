// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Contract, Counter, Stake};
use canonical::Store;
use dusk_bls12_381_sign::APK;

impl<S: Store> Contract<S> {
    pub fn find_stake(
        &self,
        w_i: Counter,
        pk: APK,
    ) -> Result<Option<Stake>, S::Error> {
        let key = self.stake_identifier_set.get(w_i)?;
        if key.is_none() {
            return Ok(None);
        }

        let key = key.unwrap();
        if key.pk == pk {
            return self.stake_mapping.get(&key);
        }

        return Ok(None);
    }
}
