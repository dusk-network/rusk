// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod circuits;
mod error;
mod state;
mod tree;

use rusk_abi::{ModuleId, State};
use state::TransferState;

#[no_mangle]
static SELF_ID: ModuleId = ModuleId::uninitialized();

static mut STATE: State<TransferState> = State::new(TransferState::new());

// Transactions

#[no_mangle]
unsafe fn execute(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| STATE.execute(arg))
}

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn stct(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| {
        STATE.send_to_contract_transparent(arg)
    })
}

#[no_mangle]
unsafe fn wfct(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| {
        STATE.withdraw_from_contract_transparent(arg)
    })
}

#[no_mangle]
unsafe fn stco(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| {
        STATE.send_to_contract_obfuscated(arg)
    })
}

#[no_mangle]
unsafe fn wfco(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| {
        STATE.withdraw_from_contract_obfuscated(arg)
    })
}

#[no_mangle]
unsafe fn wfctc(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |arg| {
        STATE.withdraw_from_contract_transparent_to_contract(arg)
    })
}

// Queries

#[no_mangle]
unsafe fn root(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |_: ()| STATE.root())
}

#[no_mangle]
unsafe fn leaves_in_range(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |(start, end)| {
        STATE.leaves_in_range(start..end)
    })
}

#[no_mangle]
unsafe fn module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |module| STATE.balance(&module))
}

#[no_mangle]
unsafe fn message(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |(module, pk)| STATE.message(&module, &pk))
}

#[no_mangle]
unsafe fn opening(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |pos| STATE.opening(pos))
}

#[no_mangle]
unsafe fn existing_nullifiers(arg_len: u32) -> u32 {
    rusk_abi::wrap_query(arg_len, |nullifiers| {
        STATE.existing_nullifiers(nullifiers)
    })
}

// "Management" transactions

#[no_mangle]
unsafe fn push_note(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(block_height, note)| {
        assert_external_caller();
        STATE.push_note(block_height, note)
    })
}

#[no_mangle]
unsafe fn update_root(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |_: ()| {
        assert_external_caller();
        STATE.update_root()
    })
}

#[no_mangle]
unsafe fn add_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_transaction(arg_len, |(module, value)| {
        assert_external_caller();
        STATE.add_balance(module, value)
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
