// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

use dusk_core::abi;

mod state;
use state::Bob;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    #[no_mangle]
    static mut STATE: Bob = Bob::new();

    #[no_mangle]
    unsafe fn init(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |n| STATE.init(n))
    }

    #[no_mangle]
    unsafe fn reset(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |n| STATE.reset(n))
    }

    #[no_mangle]
    unsafe fn owner_reset(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(sig, msg)| STATE.owner_reset(sig, msg))
    }

    #[no_mangle]
    unsafe fn ping(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |()| STATE.ping())
    }

    #[no_mangle]
    unsafe fn echo(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |n| STATE.echo(n))
    }

    #[no_mangle]
    unsafe fn value(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |()| STATE.value())
    }

    #[no_mangle]
    unsafe fn nonce(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |()| STATE.nonce())
    }

    #[no_mangle]
    unsafe fn recv_transfer(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |arg| STATE.recv_transfer(arg))
    }
}
