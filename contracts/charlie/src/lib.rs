// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]

extern crate alloc;

mod state;
use state::Charlie;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    #[no_mangle]
    static mut STATE: Charlie = Charlie;

    #[no_mangle]
    unsafe fn stake(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |stake| STATE.stake(stake))
    }
}
