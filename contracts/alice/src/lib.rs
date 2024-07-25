// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod state;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;
    use state::Alice;

    use rusk_abi::ContractId;

    #[no_mangle]
    static SELF_ID: ContractId = ContractId::uninitialized();

    static mut STATE: Alice = state::Alice;

    #[no_mangle]
    unsafe fn ping(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |()| STATE.ping())
    }

    #[no_mangle]
    unsafe fn withdraw(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |arg| STATE.withdraw(arg))
    }

    #[no_mangle]
    unsafe fn deposit(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |arg| STATE.deposit(arg))
    }
}
