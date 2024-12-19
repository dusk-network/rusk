// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

mod error;
mod state;
mod transitory;
mod tree;
mod verifier_data;

use dusk_core::abi;
use dusk_core::stake::STAKE_CONTRACT;
use state::TransferState;

static mut STATE: TransferState = TransferState::new();

// Transactions

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn mint_to_contract(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.mint_to_contract(arg))
}

#[no_mangle]
unsafe fn deposit(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.deposit(arg))
}

#[no_mangle]
unsafe fn withdraw(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.withdraw(arg))
}

#[no_mangle]
unsafe fn convert(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.convert(arg))
}

#[no_mangle]
unsafe fn contract_to_contract(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.contract_to_contract(arg))
}

#[no_mangle]
unsafe fn contract_to_account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.contract_to_account(arg))
}

// Queries

#[no_mangle]
unsafe fn root(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.root())
}

#[no_mangle]
unsafe fn account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |key| STATE.account(&key))
}

#[no_mangle]
unsafe fn contract_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |contract| STATE.contract_balance(&contract))
}

#[no_mangle]
unsafe fn opening(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |pos| STATE.opening(pos))
}

#[no_mangle]
unsafe fn existing_nullifiers(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |nullifiers| STATE.existing_nullifiers(nullifiers))
}

#[no_mangle]
unsafe fn num_notes(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.num_notes())
}

#[no_mangle]
unsafe fn chain_id(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.chain_id())
}

// "Feeder" queries

#[no_mangle]
unsafe fn leaves_from_height(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |height| STATE.leaves_from_height(height))
}

#[no_mangle]
unsafe fn leaves_from_pos(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |pos| STATE.leaves_from_pos(pos))
}

#[no_mangle]
unsafe fn sync(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(from, count_limint)| {
        STATE.sync(from, count_limint)
    })
}

#[no_mangle]
unsafe fn sync_nullifiers(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(from, count_limint)| {
        STATE.sync_nullifiers(from, count_limint)
    })
}

#[no_mangle]
unsafe fn sync_contract_balances(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(from, count_limint)| {
        STATE.sync_contract_balances(from, count_limint)
    })
}

#[no_mangle]
unsafe fn sync_accounts(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(from, count_limint)| {
        STATE.sync_accounts(from, count_limint)
    })
}

// "Management" transactions

#[no_mangle]
unsafe fn spend_and_execute(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |tx| {
        assert_external_caller();
        STATE.spend_and_execute(tx)
    })
}

#[no_mangle]
unsafe fn refund(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |gas_spent| {
        assert_external_caller();
        STATE.refund(gas_spent)
    })
}

#[no_mangle]
unsafe fn push_note(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(block_height, note)| {
        assert_external_caller();
        STATE.push_note(block_height, note)
    })
}

#[no_mangle]
unsafe fn update_root(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| {
        assert_external_caller();
        STATE.update_root()
    })
}

#[no_mangle]
unsafe fn add_account_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(key, value)| {
        assert_external_caller();
        STATE.add_account_balance(&key, value)
    })
}

#[no_mangle]
unsafe fn sub_account_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(key, value)| {
        assert_external_caller();
        STATE.sub_account_balance(&key, value)
    })
}

#[no_mangle]
unsafe fn add_contract_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(module, value)| {
        assert_external_caller();
        STATE.add_contract_balance(module, value)
    })
}

#[no_mangle]
unsafe fn sub_contract_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(module, value)| {
        assert_stake_caller();
        STATE
            .sub_contract_balance(&module, value)
            .expect("Cannot subtract balance")
    })
}

fn assert_stake_caller() {
    const PANIC_MSG: &str = "Can only be called by the stake contract";
    if abi::caller().expect(PANIC_MSG) != STAKE_CONTRACT {
        panic!("{PANIC_MSG}");
    }
}

/// Asserts the call is made "from the outside", meaning that it's not an
/// inter-contract call.
///
/// # Panics
/// When the `caller` is not "uninitialized".
fn assert_external_caller() {
    if abi::caller().is_some() {
        panic!("Can only be called from the outside the VM");
    }
}
