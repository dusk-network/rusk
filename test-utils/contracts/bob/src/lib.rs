// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

use rusk_abi::ModuleId;

#[derive(Debug, Clone)]
pub struct Bob {
    transfer: ModuleId,
}

impl Bob {
    pub const fn new(transfer: ModuleId) -> Self {
        Self { transfer }
    }
    pub fn identifier() -> &'static [u8; 3] {
        b"bob"
    }
}

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use alloc::vec::Vec;
    use dusk_pki::StealthAddress;
    use phoenix_core::{Message, Note};
    use rusk_abi::dusk::*;
    use rusk_abi::RawTransaction;
    use rusk_abi::{ModuleId, PaymentInfo, State};
    use transfer_contract_types::{Wfco2, Wfct2, Wfctc};

    const PAGE_SIZE: usize = 1024 * 4;

    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<Bob> = State::new(Bob::new(SELF_ID));

    #[no_mangle]
    unsafe fn ping(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.ping())
    }

    #[no_mangle]
    unsafe fn payment_info(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(arg_len, |()| STATE.payment_info())
    }

    impl Bob {
        pub fn ping(&mut self) {
            rusk_abi::debug!("Bob ping");
        }

        pub fn payment_info(self: &mut State<Self>) {
            rusk_abi::debug!("Bob payment_info");
        }
    }
}
