// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod error;
mod license_circuits;
mod license_types;
mod state;
use license_types::*;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use rusk_abi::{ModuleId, State};
    use state::LicensesData;

    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<LicensesData> = State::new(LicensesData::new());

    #[no_mangle]
    unsafe fn request_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |license_request| {
            STATE.request_license(license_request)
        })
    }

    #[no_mangle]
    unsafe fn get_license_request(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |sp_public_key| {
            STATE.get_license_request(sp_public_key)
        })
    }

    #[no_mangle]
    unsafe fn issue_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |license| {
            STATE.issue_license(license)
        })
    }

    #[no_mangle]
    unsafe fn get_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |user_public_key| {
            STATE.get_license(user_public_key)
        })
    }

    #[no_mangle]
    unsafe fn use_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(arg_len, |use_license_arg| {
            STATE.use_license(use_license_arg)
        })
    }

    #[no_mangle]
    unsafe fn get_session(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |nullifier| STATE.get_session(nullifier))
    }
}
