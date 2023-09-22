// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_family = "wasm", no_std)]
#![cfg(target_family = "wasm")]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod error;
mod license_circuits;
mod license_types;
#[cfg(target_family = "wasm")]
mod state;

pub use license_types::{
    LicenseSession, LicenseSessionId, PoseidonItem, UseLicenseArg,
};

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use rusk_abi::ContractId;
    use state::LicenseContractState;

    #[no_mangle]
    static SELF_ID: ContractId = ContractId::uninitialized();

    static mut STATE: LicenseContractState = LicenseContractState::new();

    #[no_mangle]
    unsafe fn issue_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |(license, hash)| {
            STATE.issue_license(license, hash)
        })
    }

    #[no_mangle]
    unsafe fn get_licenses(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |user_public_key| {
            STATE.get_licenses(user_public_key)
        })
    }

    #[no_mangle]
    unsafe fn get_merkle_opening(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |position| {
            STATE.get_merkle_opening(position)
        })
    }

    #[no_mangle]
    unsafe fn use_license(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |use_license_arg| {
            STATE.use_license(use_license_arg)
        })
    }

    #[no_mangle]
    unsafe fn get_session(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |nullifier| STATE.get_session(nullifier))
    }

    #[no_mangle]
    unsafe fn noop(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |_: ()| STATE.noop())
    }

    #[no_mangle]
    unsafe fn get_info(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |_: ()| STATE.get_info())
    }
}
