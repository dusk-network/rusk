// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod collection;
mod msg;
mod state;

use msg::*;
use rusk_abi::State;
use state::GovernanceState;

static mut STATE: State<GovernanceState> = State::new(GovernanceState::new());

// Transactions

#[no_mangle]
unsafe fn transfer(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed, batch)| {
        let msg = transfer_msg(seed, &batch);
        STATE.assert_signature(signature, seed, msg);
        STATE.transfer(batch)
    })
}

#[no_mangle]
unsafe fn fee(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed, batch)| {
        let msg = fee_msg(seed, &batch);
        STATE.assert_signature(signature, seed, msg);
        STATE.fee(batch)
    })
}

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed, address, amount)| {
        let msg = mint_msg(seed, address, amount);
        STATE.assert_signature(signature, seed, msg);
        STATE.mint(address, amount)
    })
}

#[no_mangle]
unsafe fn burn(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed, address, amount)| {
        let msg = burn_msg(seed, address, amount);
        STATE.assert_signature(signature, seed, msg);
        STATE.burn(address, amount)
    })
}

#[no_mangle]
unsafe fn pause(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed)| {
        let msg = pause_msg(seed);
        STATE.assert_signature(signature, seed, msg);
        STATE.pause()
    })
}

#[no_mangle]
unsafe fn unpause(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(signature, seed)| {
        let msg = unpause_msg(seed);
        STATE.assert_signature(signature, seed, msg);
        STATE.unpause()
    })
}

// Queries

#[no_mangle]
unsafe fn balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |address| STATE.balance(&address))
}

#[no_mangle]
unsafe fn total_supply(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| STATE.total_supply())
}

#[no_mangle]
unsafe fn authority(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| STATE.get_authority())
}

#[no_mangle]
unsafe fn broker(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| STATE.get_broker())
}

// "Management" transactions

#[no_mangle]
unsafe fn set_authority(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |authority| {
        assert_external_caller();
        STATE.set_authority(authority)
    })
}

#[no_mangle]
unsafe fn set_broker(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |broker| {
        assert_external_caller();
        STATE.set_broker(broker)
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
