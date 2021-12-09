// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The foreign function interface for the wallet.

use core::num::NonZeroU32;

use dusk_bytes::Serializable;
use dusk_pki::SecretSpendKey;
use rand_core::{CryptoRng, RngCore};

use crate::Store;

extern "C" {
    fn wallet_store(
        id: *const u8,
        id_len: u32,
        key: *const [u8; SecretSpendKey::SIZE],
    ) -> u8;
    fn wallet_load(
        id: *const u8,
        id_len: u32,
        key: *mut [u8; SecretSpendKey::SIZE],
    ) -> u8;
    fn wallet_delete(id: *const u8, id_len: u32) -> u8;
    fn fill_random(buf: *mut u8, buf_len: u32) -> u8;
}

macro_rules! error_if_not_zero {
    ($e: expr) => {
        match $e {
            0 => {}
            c => return Err(c),
        }
    };
}

struct FfiStore;

impl Store for FfiStore {
    type Id = String;
    type Error = u8;

    fn store(
        &mut self,
        id: &Self::Id,
        key: &SecretSpendKey,
    ) -> Result<(), Self::Error> {
        let buf = key.to_bytes();
        unsafe {
            error_if_not_zero!(wallet_store(
                &id.as_bytes()[0],
                id.len() as u32,
                &buf
            ));
        }
        Ok(())
    }

    fn load(
        &self,
        id: &Self::Id,
    ) -> Result<Option<SecretSpendKey>, Self::Error> {
        let mut buf = [0u8; SecretSpendKey::SIZE];
        unsafe {
            error_if_not_zero!(wallet_load(
                &id.as_bytes()[0],
                id.len() as u32,
                &mut buf
            ));
        }
        Ok(SecretSpendKey::from_bytes(&buf).ok())
    }

    fn delete(&mut self, id: &Self::Id) -> Result<(), Self::Error> {
        unsafe {
            error_if_not_zero!(wallet_delete(
                &id.as_bytes()[0],
                id.len() as u32
            ));
        }
        Ok(())
    }
}

struct FfiRng;

impl CryptoRng for FfiRng {}

impl RngCore for FfiRng {
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        self.fill_bytes(&mut buf);
        u32::from_ne_bytes(buf)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        self.fill_bytes(&mut buf);
        u64::from_ne_bytes(buf)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).ok();
    }

    fn try_fill_bytes(
        &mut self,
        dest: &mut [u8],
    ) -> Result<(), rand_core::Error> {
        let buf = dest.as_mut_ptr();
        let len = dest.len();

        // SAFETY: this is unsafe since the passed function is not guaranteed to
        // be a CSPRNG running in a secure context. We therefore consider it the
        // responsibility of the user to pass a good generator.
        unsafe {
            match fill_random(buf, len as u32) {
                0 => Ok(()),
                v => {
                    let nzu = NonZeroU32::new(v as u32).unwrap();
                    Err(rand_core::Error::from(nzu))
                }
            }
        }
    }
}
