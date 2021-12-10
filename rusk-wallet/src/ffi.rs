// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The foreign function interface for the wallet.

use core::num::NonZeroU32;
use core::slice;

use dusk_bytes::Serializable;
use dusk_pki::SecretSpendKey;
use rand_core::{CryptoRng, RngCore};

use crate::{Error, Store, Wallet};

extern "C" {
    fn store_key(
        id: *const u8,
        id_len: u32,
        key: *const [u8; SecretSpendKey::SIZE],
    ) -> u8;
    fn get_key(
        id: *const u8,
        id_len: u32,
        key: *mut [u8; SecretSpendKey::SIZE],
    ) -> u8;
    fn fill_random(buf: *mut u8, buf_len: u32) -> u8;
}

macro_rules! error_if_not_zero {
    ($e: expr) => {
        if $e != 0 {
            return Err($e);
        }
    };
}

macro_rules! unwrap_or_bail {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return Error::<FfiStore>::from(e).into();
            }
        }
    };
}

const FFI_WALLET: Wallet<FfiStore> = Wallet::new(FfiStore);

/// Create a secret spend key.
#[no_mangle]
pub unsafe extern "C" fn create_ssk(id: *const u8, id_len: u32) -> u8 {
    let id = slice::from_raw_parts(id, id_len as usize);
    let id = String::from_utf8_unchecked(id.to_vec());

    unwrap_or_bail!(FFI_WALLET.create_ssk(&mut FfiRng, &id));

    0
}

/// Loads a secret spend key into the wallet.
#[no_mangle]
pub unsafe extern "C" fn load_ssk(
    id: *const u8,
    id_len: u32,
    ssk: *const [u8; SecretSpendKey::SIZE],
) -> u8 {
    let id = slice::from_raw_parts(id, id_len as usize);
    let id = String::from_utf8_unchecked(id.to_vec());

    let ssk = unwrap_or_bail!(SecretSpendKey::from_bytes(&*ssk));
    unwrap_or_bail!(FFI_WALLET.load_ssk(&id, &ssk));

    0
}

/// Creates a transfer transaction.
#[no_mangle]
pub unsafe extern "C" fn create_transfer_tx() {
    todo!()
}

/// Creates a stake transaction.
#[no_mangle]
pub unsafe extern "C" fn create_stake_tx() {
    todo!()
}

/// Stops staking for a key.
#[no_mangle]
pub unsafe extern "C" fn stop_stake() {
    todo!()
}

/// Extends staking for a particular key.
#[no_mangle]
pub unsafe extern "C" fn extend_stake() {
    todo!()
}

/// Withdraw a key's stake.
#[no_mangle]
pub unsafe extern "C" fn withdraw_stake() {
    todo!()
}

/// Syncs the wallet with the blocks.
#[no_mangle]
pub unsafe extern "C" fn sync() {
    todo!()
}

/// Gets the balance of a key.
#[no_mangle]
pub unsafe extern "C" fn get_balance() {
    todo!()
}

struct FfiStore;

impl Store for FfiStore {
    type Id = String;
    type Error = u8;

    fn store_key(
        &self,
        id: &Self::Id,
        key: &SecretSpendKey,
    ) -> Result<(), Self::Error> {
        let buf = key.to_bytes();
        unsafe {
            error_if_not_zero!(store_key(
                &id.as_bytes()[0],
                id.len() as u32,
                &buf
            ));
        }
        Ok(())
    }

    fn key(
        &self,
        id: &Self::Id,
    ) -> Result<Option<SecretSpendKey>, Self::Error> {
        let mut buf = [0u8; SecretSpendKey::SIZE];
        unsafe {
            error_if_not_zero!(get_key(
                &id.as_bytes()[0],
                id.len() as u32,
                &mut buf
            ));
        }
        Ok(SecretSpendKey::from_bytes(&buf).ok())
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

impl<S: Store> From<Error<S>> for u8 {
    fn from(e: Error<S>) -> Self {
        match e {
            Error::Store(_) => 1,
            Error::Rng(_) => 2,
            Error::Bytes(_) => 3,
        }
    }
}
