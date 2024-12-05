// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

mod state;
use execution_core::transfer::TRANSFER_CONTRACT;
use state::Charlie;

#[no_mangle]
static mut STATE: Charlie = Charlie;

#[no_mangle]
unsafe fn stake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |stake| STATE.stake(stake))
}

#[no_mangle]
unsafe fn unstake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |withdraw| STATE.unstake(withdraw))
}

#[no_mangle]
unsafe fn receive_unstake(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |withdraw| {
        assert_transfer_caller();
        // Assert is not called directly by "spend_and_execute"
        // (it's supposed to be called by
        // TRANSFER_CONTRACT::contract_to_contract ICC)
        if rusk_abi::callstack().len() < 2 {
            panic!("Cannot be called by a root ICC")
        }
        STATE.receive_unstake(withdraw)
    })
}

#[no_mangle]
unsafe fn withdraw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |withdraw| STATE.withdraw(withdraw))
}

#[no_mangle]
unsafe fn receive_reward(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |withdraw| {
        assert_transfer_caller();
        // Assert is not called directly by "spend_and_execute"
        // (it's supposed to be called by
        // TRANSFER_CONTRACT::contract_to_contract ICC)
        if rusk_abi::callstack().len() < 2 {
            panic!("Cannot be called by a root ICC")
        }
        STATE.receive_reward(withdraw)
    })
}

fn assert_transfer_caller() {
    const PANIC_MSG: &str = "Can only be called from the transfer contract";
    if rusk_abi::caller().expect(PANIC_MSG) != TRANSFER_CONTRACT {
        panic!("{PANIC_MSG}");
    }
}
