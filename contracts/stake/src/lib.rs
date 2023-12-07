// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

use rusk_abi::dusk::*;

mod state;
use state::StakeState;

/// The minimum amount of Dusk one can stake.
pub const MINIMUM_STAKE: Dusk = dusk(1_000.0);

use dusk_bls12_381_sign::PublicKey;
use rusk_abi::{ContractId, PaymentInfo};

#[no_mangle]
static SELF_ID: ContractId = ContractId::uninitialized();

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

#[no_mangle]
unsafe fn allow(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| {
        assert_transfer_caller();
        STATE.allow(arg)
    })
}

// Queries

#[no_mangle]
unsafe fn get_stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk: PublicKey| {
        STATE.get_stake(&pk).cloned().map(|s| s.0)
    })
}

#[no_mangle]
unsafe fn allowlist(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.stakers_allowlist())
}

#[no_mangle]
unsafe fn is_allowlisted(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk| STATE.is_allowlisted(&pk))
}

#[no_mangle]
unsafe fn owners(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.owners())
}

// "Feeder" queries

#[no_mangle]
unsafe fn stakes(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| STATE.stakes())
}

// "Management" transactions

#[no_mangle]
unsafe fn insert_stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pk, stake_data)| {
        assert_external_caller();
        STATE.insert_stake(pk, stake_data)
    })
}

#[no_mangle]
unsafe fn insert_allowlist(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk| {
        assert_external_caller();
        STATE.insert_allowlist(pk);
    })
}

#[no_mangle]
unsafe fn reward(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |(pk, value)| {
        assert_external_caller();
        STATE.reward(&pk, value);
    })
}

#[no_mangle]
unsafe fn add_owner(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |pk| {
        assert_external_caller();
        STATE.add_owner(pk);
    })
}

/// Asserts the call is made via the transfer contract.
///
/// # Panics
/// When the `caller` is not [`rusk_abi::TRANSFER_CONTRACT`].
fn assert_transfer_caller() {
    if rusk_abi::caller() != rusk_abi::TRANSFER_CONTRACT {
        panic!("Can only be called from the transfer contract");
    }
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

const PAYMENT_INFO: PaymentInfo = PaymentInfo::Transparent(None);

#[no_mangle]
fn payment_info(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |_: ()| PAYMENT_INFO)
}
