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

use rusk_abi::{ContractId, STAKE_CONTRACT};
use state::TransferState;

#[no_mangle]
static SELF_ID: ContractId = ContractId::uninitialized();

static mut STATE: TransferState = TransferState::new();

// Transactions

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn stct(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.send_to_contract_transparent(arg))
}

#[no_mangle]
unsafe fn wfct(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        STATE.withdraw_from_contract_transparent(arg)
    })
}

#[no_mangle]
unsafe fn wfct_raw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        STATE.withdraw_from_contract_transparent_raw(arg)
    })
}

#[no_mangle]
unsafe fn stco(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.send_to_contract_obfuscated(arg))
}

#[no_mangle]
unsafe fn wfco(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        STATE.withdraw_from_contract_obfuscated(arg)
    })
}

#[no_mangle]
unsafe fn wfco_raw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        STATE.withdraw_from_contract_obfuscated_raw(arg)
    })
}

#[no_mangle]
unsafe fn wfctc(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        STATE.withdraw_from_contract_transparent_to_contract(arg)
    })
}

// Queries

#[no_mangle]
unsafe fn root(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.root())
}

#[no_mangle]
unsafe fn module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |module| STATE.balance(&module))
}

#[no_mangle]
unsafe fn message(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, pk)| STATE.message(&module, &pk))
}

#[no_mangle]
unsafe fn opening(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pos| STATE.opening(pos))
}

#[no_mangle]
unsafe fn existing_nullifiers(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |nullifiers| {
        STATE.existing_nullifiers(nullifiers)
    })
}

#[no_mangle]
unsafe fn num_notes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.num_notes())
}

// "Feeder" queries

#[no_mangle]
unsafe fn leaves_from_height(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |height| STATE.leaves_from_height(height))
}

#[no_mangle]
unsafe fn leaves_from_pos(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pos| STATE.leaves_from_pos(pos))
}

// "Management" transactions

#[no_mangle]
unsafe fn spend_and_execute(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |tx| {
        assert_external_caller();
        STATE.spend_and_execute(tx)
    })
}

#[no_mangle]
unsafe fn refund(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(fee, gas_spent)| {
        assert_external_caller();
        STATE.refund(fee, gas_spent)
    })
}

#[no_mangle]
unsafe fn push_note(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(block_height, note)| {
        assert_external_caller();
        STATE.push_note(block_height, note)
    })
}

#[no_mangle]
unsafe fn update_root(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| {
        assert_external_caller();
        STATE.update_root()
    })
}

#[no_mangle]
unsafe fn add_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, value)| {
        assert_external_caller();
        STATE.add_balance(module, value)
    })
}

#[no_mangle]
unsafe fn sub_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(module, value)| {
        if rusk_abi::caller() != STAKE_CONTRACT {
            panic!("Can only be called by the stake contract!")
        }
        STATE
            .sub_balance(&module, value)
            .expect("Cannot subtract balance")
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
