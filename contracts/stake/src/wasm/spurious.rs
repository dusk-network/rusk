// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::stake::{Counter, Stake, StakeContract};
use canonical::Store;
use dusk_bls12_381_sign::APK;

impl<S: Store> StakeContract<S> {
    pub fn find_stake(
        &self,
        w_i: Counter,
        pk: APK,
    ) -> Result<Option<Stake>, S::Error> {
        let key = match self.stake_identifier_set.get(&w_i) {
            Ok(Some(k)) => k,
            Err(e) => return Err(e),
            _ => return Ok(None),
        };

        if key.pk == pk {
            match self.stake_mapping.get(&key) {
                Ok(Some(stake)) => return Ok(Some(*stake)),
                Err(e) => return Err(e),
                _ => return Ok(None),
            }
        }

        Ok(None)
    }
}
