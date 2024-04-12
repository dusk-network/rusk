// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod state;
use state::Charlie;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use rusk_abi::{ContractId, PaymentInfo};

    #[no_mangle]
    static SELF_ID: ContractId = ContractId::uninitialized();

    static mut STATE: Charlie = Charlie;

    #[no_mangle]
    unsafe fn pay(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |()| STATE.pay())
    }

    #[no_mangle]
    unsafe fn pay_and_fail(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |()| STATE.pay_and_fail())
    }

    #[no_mangle]
    unsafe fn earn(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |()| STATE.earn())
    }

    #[no_mangle]
    unsafe fn earn_and_fail(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |()| STATE.earn_and_fail())
    }

    #[no_mangle]
    unsafe fn subsidize(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |arg| STATE.subsidize(arg))
    }

    const PAYMENT_INFO: PaymentInfo = PaymentInfo::Any(None);

    #[no_mangle]
    fn payment_info(arg_len: u32) -> u32 {
        rusk_abi::wrap_call(arg_len, |_: ()| PAYMENT_INFO)
    }
}
