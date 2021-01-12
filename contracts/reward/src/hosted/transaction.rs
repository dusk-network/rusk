// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Contract, Key, PublicKeys};
use canonical::Store;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature, APK};

/// TODO: Still waiting for values from the research side.
/// t_w in the specs
const WITHDRAWAL_PERIOD: u64 = 0;

extern "C" {
    fn verify_bls_sig(pk: &u8, sig: &u8, msg: &u8) -> i32;
}

impl<S: Store> Contract<S> {
    pub fn distribute(
        &mut self,
        value: u64,
        public_keys: PublicKeys<S>,
    ) -> bool {
        match public_keys
            .pks
            .iter()
            .map(|pk| {
                canonical::debug!("new iteration");
                let mut values: (u64, u64);
                match self.balance_mapping.get(pk.clone()) {
                    Ok(Some(v)) => values = *v,
                    _ => values = (0, 0),
                }

                values.0 = values.0 + value;
                self.balance_mapping.insert(*pk, values.0, values.1)?;
                Ok(())
            })
            .collect::<Result<(), S::Error>>()
        {
            Ok(_) => true,
            _ => false,
        }
    }

    pub fn withdraw(
        &mut self,
        block_height: u64,
        pk: APK,
        sig: Signature,
    ) -> bool {
        let mut values: (u64, u64);
        match self.balance_mapping.get(pk) {
            Ok(Some(v)) => values = *v,
            _ => return false,
        }

        // TODO: Check that note value is less than or equal to mapped value

        if values.1 + WITHDRAWAL_PERIOD > block_height {
            return false;
        }

        let mut msg_bytes = [0u8; 64];
        // TODO: input note value to msg_bytes
        msg_bytes[32..64]
            .copy_from_slice(&BlsScalar::from(values.1).to_bytes());
        let pk_bytes = pk.to_bytes();
        let sig_bytes = sig.to_bytes();

        // Verify BLS sig.
        let res = unsafe {
            verify_bls_sig(&pk_bytes[0], &sig_bytes[0], &msg_bytes[0])
        };

        if res == 0i32 {
            return false;
        }

        // TODO: subtracts note value from values.0
        values.1 = block_height;

        // TODO: intercontract call

        match self.balance_mapping.insert(pk, values.0, values.1) {
            Ok(_) => true,
            _ => false,
        }
    }
}
