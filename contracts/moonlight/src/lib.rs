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
use state::MoonlightState;

static mut STATE: MoonlightState = MoonlightState::new();

#[no_mangle]
unsafe fn deposit(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.deposit(arg))
}

#[no_mangle]
unsafe fn transfer(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.transfer(arg))
}

#[no_mangle]
unsafe fn withdraw(arg_len: u32) -> u32 {
    rusk_abi::wrap_call(arg_len, |arg| STATE.withdraw(arg))
}
