// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![feature(arbitrary_self_types)]

use rusk_abi::dusk::*;

pub type BlockHeight = u64;

/// Epoch used for stake operations
pub const EPOCH: u64 = 2160;

/// Maturity of the stake
pub const MATURITY: u64 = 2 * EPOCH;

/// The minimum amount of Dusk one can stake.
pub const MINIMUM_STAKE: Dusk = dusk(1_000.0);

extern crate alloc;

// TODO: This is to allow the tests to run, since this library
// is supposed to be compiled (and working) only for WASM
// However, we should move the test outside this crate
// most likely in one of the ancestors.
#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    pub mod contract;
    pub mod stake;
    pub mod error;
    pub mod wasm;

    use rusk_abi::{ModuleId, State};
    use contract::StakeContract;
    pub use stake::Stake;
    pub use error::Error;
    pub use wasm::*;


    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<StakeContract> = State::new(StakeContract::new());

    // Transactions

    #[no_mangle]
    unsafe fn stake(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |(pk, signature, value, spend_proof)| STATE.stake(pk, signature, value, spend_proof))
    }

    #[no_mangle]
    unsafe fn unstake(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |(pk, signature, note, withdraw_proof)| STATE.unstake(pk, signature, note, withdraw_proof))
    }

    #[no_mangle]
    unsafe fn withdraw(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |(pk, signature, address, nonce)| STATE.withdraw(pk, signature, address, nonce))
    }

    #[no_mangle]
    unsafe fn allowlist(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |(pk, signature, owner)| STATE.allowlist(pk, signature, owner))
    }
}
