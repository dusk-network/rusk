// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

mod state;
use state::License;

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use rusk_abi::{PaymentInfo, State};

    #[no_mangle]
    static mut STATE: State<License> = State::new(License::new());

    #[no_mangle]
    unsafe fn ping(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.ping())
    }

    const PAYMENT_INFO: PaymentInfo = PaymentInfo::Transparent(None);

    #[no_mangle]
    fn payment_info(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |_: ()| PAYMENT_INFO)
    }
}
