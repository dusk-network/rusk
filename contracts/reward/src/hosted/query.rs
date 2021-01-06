// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Contract;
use canonical::Store;
use dusk_bls12_381_sign::APK;

impl<S: Store> Contract<S> {
    pub fn get_balance(&self, pk: APK) -> Result<Option<u64>, S::Error> {
        let values = self.balance_mapping.get(pk)?;
        if values.is_none() {
            return Ok(None);
        }

        let values = values.unwrap();
        return Ok(Some(values.0));
    }

    pub fn get_withdrawal_time(
        &self,
        pk: APK,
    ) -> Result<Option<u64>, S::Error> {
        let values = self.balance_mapping.get(pk)?;
        if values.is_none() {
            return Ok(None);
        }

        let values = values.unwrap();
        return Ok(Some(values.1));
    }
}
