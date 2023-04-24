// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod state;
mod license_types;
use state::{License};
use license_types::*;


#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use rusk_abi::{ModuleId, State};

    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<License> = State::new(License::new());

    #[no_mangle]
    unsafe fn request_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.request_license())
    }

    #[no_mangle]
    unsafe fn get_license_request(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.get_license_request())
    }

    #[no_mangle]
    unsafe fn issue_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.issue_license())
    }

    #[no_mangle]
    unsafe fn get_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.get_license())
    }

    #[no_mangle]
    unsafe fn use_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.use_license())
    }

    #[no_mangle]
    unsafe fn get_session(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |nullifier| STATE.get_session(nullifier))
    }
}
