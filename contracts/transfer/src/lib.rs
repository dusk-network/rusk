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

use rusk_abi::{ContractId, STAKE_CONTRACT};
use state::TransferOps;

#[no_mangle]
static SELF_ID: ContractId = ContractId::uninitialized();

static mut STATE: TransferOps = TransferOps {};

// Transactions

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.mint(arg)
    })
}

#[no_mangle]
unsafe fn stct(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.send_to_contract_transparent(arg)
    })
}

#[no_mangle]
unsafe fn wfct(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(arg, from_address)| {
        assert_transfer_caller();
        STATE.withdraw_from_contract_transparent(arg, from_address)
    })
}

#[no_mangle]
unsafe fn wfct_raw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(arg, from_address)| {
        assert_transfer_caller();
        STATE.withdraw_from_contract_transparent_raw(arg, from_address)
    })
}

#[no_mangle]
unsafe fn stco(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.send_to_contract_obfuscated(arg)
    })
}

#[no_mangle]
unsafe fn wfco(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(arg, from_address)| {
        assert_transfer_caller();
        STATE.withdraw_from_contract_obfuscated(arg, from_address)
    })
}

#[no_mangle]
unsafe fn wfco_raw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(arg, from_address)| {
        assert_transfer_caller();
        STATE.withdraw_from_contract_obfuscated_raw(arg, from_address)
    })
}

#[no_mangle]
unsafe fn wfctc(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(arg, from_address)| {
        assert_transfer_caller();
        STATE.withdraw_from_contract_transparent_to_contract(arg, from_address)
    })
}

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
        STATE.existing_nullifiers(&nullifiers)
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
unsafe fn spend(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |tx| {
        assert_transfer_caller();
        STATE.spend(tx)
    })
}

#[no_mangle]
unsafe fn execute(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |tx| {
        assert_transfer_caller();
        STATE.execute(tx)
    })
}

#[no_mangle]
unsafe fn refund(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(fee, gas_spent)| {
        assert_transfer_caller();
        STATE.refund(fee, gas_spent)
    })
}

#[no_mangle]
unsafe fn push_note(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(block_height, note)| {
        assert_transfer_caller();
        STATE.push_note(block_height, note)
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

/// Asserts the call is made via the transfer contract.
#[no_mangle]
unsafe fn sub_module_balance(arg_len: u32) -> u32 {
    rusk_abi::wrap_call::<(ContractId, u64, ContractId), (), _>(
        arg_len,
        |(module, value, caller)| {
            if caller != STAKE_CONTRACT {
                panic!("Can only be called by the stake contract!")
            }
            STATE
                .sub_balance(&module, value)
                .expect("Cannot subtract balance")
        },
    )
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
