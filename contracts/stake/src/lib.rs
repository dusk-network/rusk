// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![feature(arbitrary_self_types)]

extern crate alloc;

use rusk_abi::dusk::*;

mod state;
use state::StakeState;

/// The minimum amount of Dusk one can stake.
pub const MINIMUM_STAKE: Dusk = dusk(1_000.0);

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use dusk_bls12_381_sign::APK;
    use rusk_abi::{ModuleId, State};

    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<StakeState> = State::new(StakeState::new());

    // Transactions

    #[no_mangle]
    unsafe fn stake(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |arg| STATE.stake(arg))
    }

    #[no_mangle]
    unsafe fn unstake(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |arg| STATE.unstake(arg))
    }

    #[no_mangle]
    unsafe fn withdraw(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |arg| STATE.withdraw(arg))
    }

    #[no_mangle]
    unsafe fn allow(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |arg| STATE.allow(arg))
    }

    // Queries

    #[no_mangle]
    unsafe fn get_stake(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |apk: APK| {
            STATE.get_stake(&apk).map(|data| data.clone())
        })
    }

    #[no_mangle]
    unsafe fn stakes(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |_: ()| STATE.stakes())
    }

    #[no_mangle]
    unsafe fn allowlist(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |_: ()| STATE.stakers_allowlist())
    }

    #[no_mangle]
    unsafe fn owners(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |_: ()| STATE.owners())
    }

    // "Management" transactions

    #[no_mangle]
    unsafe fn insert_stake(arg_len: u32) -> u32 {
        assert_external_caller();

        rusk_abi::wrap_transaction(arg_len, |(apk, stake_data)| {
            STATE.insert_stake(apk, stake_data);
        })
    }

    #[no_mangle]
    unsafe fn insert_allowlist(arg_len: u32) -> u32 {
        assert_external_caller();

        rusk_abi::wrap_transaction(arg_len, |apk| {
            STATE.insert_allowlist(apk);
        })
    }

    #[no_mangle]
    unsafe fn reward(arg_len: u32) -> u32 {
        assert_external_caller();

        rusk_abi::wrap_transaction(arg_len, |(apk, value): (APK, u64)| {
            STATE.reward(&apk, value);
        })
    }

    #[no_mangle]
    unsafe fn add_owner(arg_len: u32) -> u32 {
        assert_external_caller();

        rusk_abi::wrap_transaction(arg_len, |apk| {
            STATE.add_owner(apk);
        })
    }

    /// Asserts the call is made "from the outside", meaning that it's not an
    /// inter-contract call.
    ///
    /// # Panics
    /// When the `caller` is not "uninitialized".
    fn assert_external_caller() {
        if !rusk_abi::caller().is_uninitialized() {
            panic!("Can only be called from the outside the VM");
        }
    }
}
