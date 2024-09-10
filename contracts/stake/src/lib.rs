// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

use execution_core::transfer::TRANSFER_CONTRACT;

mod state;
use state::StakeState;

static mut STATE: StakeState = StakeState::new();

// Transactions

#[no_mangle]
unsafe fn stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.stake(arg)
    })
}

#[no_mangle]
unsafe fn unstake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.unstake(arg)
    })
}

#[no_mangle]
unsafe fn withdraw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.withdraw(arg)
    })
}

// Queries

#[no_mangle]
unsafe fn get_stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk| STATE.get_stake(&pk).cloned())
}

#[no_mangle]
unsafe fn get_stake_keys(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk| STATE.get_stake_keys(&pk).cloned())
}

#[no_mangle]
unsafe fn burnt_amount(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.burnt_amount())
}

#[no_mangle]
unsafe fn get_version(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.get_version())
}

// "Feeder" queries

#[no_mangle]
unsafe fn stakes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.stakes())
}

#[no_mangle]
unsafe fn prev_state_changes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.prev_state_changes())
}

// "Management" transactions

#[no_mangle]
unsafe fn before_state_transition(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_external_caller();
        STATE.on_new_block()
    })
}

#[no_mangle]
unsafe fn insert_stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pk, stake_data)| {
        assert_external_caller();
        STATE.insert_stake(pk, stake_data)
    })
}

#[no_mangle]
unsafe fn reward(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_external_caller();
        STATE.reward(arg);
    })
}

#[no_mangle]
unsafe fn slash(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pk, value)| {
        assert_external_caller();
        STATE.slash(&pk, value);
    })
}

#[no_mangle]
unsafe fn hard_slash(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pk, value, severity)| {
        assert_external_caller();
        STATE.hard_slash(&pk, value, severity);
    })
}

#[no_mangle]
unsafe fn set_burnt_amount(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |burnt_amount| {
        assert_external_caller();
        STATE.set_burnt_amount(burnt_amount)
    })
}

/// Asserts the call is made via the transfer contract.
///
/// # Panics
/// When the `caller` is not [`TRANSFER_CONTRACT`].
fn assert_transfer_caller() {
    const PANIC_MSG: &str = "Can only be called from the transfer contract";
    if rusk_abi::caller().expect(PANIC_MSG) != TRANSFER_CONTRACT {
        panic!("{PANIC_MSG}");
    }
}

/// Asserts the call is made "from the outside", meaning that it's not an
/// inter-contract call.
///
/// # Panics
/// When the `caller` is not "uninitialized".
fn assert_external_caller() {
    if rusk_abi::caller().is_some() {
        panic!("Can only be called from the outside the VM");
    }
}
