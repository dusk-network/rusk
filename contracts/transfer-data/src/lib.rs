// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod state;
mod tree;

use rusk_abi::ContractId;
use state::TransferState;

#[no_mangle]
static SELF_ID: ContractId = ContractId::uninitialized();

static mut STATE: TransferState = TransferState::new();

// Queries

#[no_mangle]
unsafe fn root(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_transfer_caller();
        STATE.root()
    })
}

#[no_mangle]
unsafe fn num_notes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_transfer_caller();
        STATE.num_notes()
    })
}

#[no_mangle]
unsafe fn module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |module| {
        assert_transfer_caller();
        STATE.balance(&module)
    })
}

#[no_mangle]
unsafe fn message(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, pk)| {
        assert_transfer_caller();
        STATE.message(&module, &pk)
    })
}

#[no_mangle]
unsafe fn opening(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pos| {
        assert_transfer_caller();
        STATE.opening(pos)
    })
}

#[no_mangle]
unsafe fn existing_nullifiers(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |nullifiers| {
        assert_transfer_caller();
        STATE.existing_nullifiers(nullifiers)
    })
}

#[no_mangle]
unsafe fn any_nullifier_exists(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |nullifiers| {
        assert_transfer_caller();
        STATE.any_nullifier_exists(nullifiers)
    })
}

#[no_mangle]
unsafe fn extend_nullifiers(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |nullifiers| {
        assert_transfer_caller();
        STATE.extend_nullifiers(nullifiers)
    })
}

#[no_mangle]
unsafe fn take_message_from_address_key(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, pk)| {
        assert_transfer_caller();
        STATE.take_message_from_address_key(&module, &pk)
    })
}

#[no_mangle]
unsafe fn root_exists(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |root| {
        assert_transfer_caller();
        STATE.root_exists(&root)
    })
}

#[no_mangle]
unsafe fn push_message(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, stealth_address, message)| {
        assert_transfer_caller();
        STATE.push_message(module, stealth_address, message)
    })
}

#[no_mangle]
unsafe fn take_crossover(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_transfer_caller();
        STATE.take_crossover()
    })
}

#[no_mangle]
unsafe fn set_crossover(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(crossover, address)| {
        assert_transfer_caller();
        STATE.set_crossover(crossover, address)
    })
}

#[no_mangle]
unsafe fn get_crossover(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_transfer_caller();
        STATE.get_crossover()
    })
}

#[no_mangle]
unsafe fn extend_notes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(height, notes)| {
        assert_transfer_caller();
        STATE.extend_notes(height, notes)
    })
}

#[no_mangle]
unsafe fn sub_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, value)| {
        assert_transfer_caller();
        STATE.sub_balance(module, value)
    })
}

// "Feeder" queries

#[no_mangle]
unsafe fn leaves_from_height(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |height| {
        assert_transfer_caller();
        STATE.leaves_from_height(height)
    })
}

#[no_mangle]
unsafe fn leaves_from_pos(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pos| {
        assert_transfer_caller();
        STATE.leaves_from_pos(pos)
    })
}

// "Management" transactions

#[no_mangle]
unsafe fn push_note(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(block_height, note)| {
        assert_transfer_caller();
        STATE.push_note(block_height, note)
    })
}

#[no_mangle]
unsafe fn get_note(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pos| {
        assert_transfer_caller();
        STATE.get_note(pos)
    })
}

#[no_mangle]
unsafe fn update_root(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_transfer_caller();
        STATE.update_root()
    })
}

#[no_mangle]
unsafe fn add_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, value)| {
        assert_transfer_caller();
        STATE.add_balance(module, value)
    })
}

#[no_mangle]
unsafe fn get_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |module| {
        assert_transfer_caller();
        STATE.balance(&module)
    })
}

/// Asserts the call is made via the transfer contract.
///
/// # Panics
/// When the `caller`s owner is not transfer contract's owner.
fn assert_transfer_caller() {
    let transfer_owner =
        rusk_abi::owner_raw(rusk_abi::TRANSFER_CONTRACT).unwrap();
    let caller_id = rusk_abi::caller();
    match rusk_abi::owner_raw(caller_id) {
        Some(caller_owner) if caller_owner.eq(&transfer_owner) => (),
        _ => panic!("Can only be called from the transfer contract"),
    }
}
