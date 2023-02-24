// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

#[cfg(target_arch = "wasm32")]
mod wasm;

mod collection;
mod error;
mod governance;

use canonical_derive::Canon;
use dusk_pki::PublicKey;

pub use error::Error;
pub use governance::GovernanceContract;

pub const TX_PAUSE: u8 = 0x00;
pub const TX_UNPAUSE: u8 = 0x01;
pub const TX_MINT: u8 = 0x02;
pub const TX_BURN: u8 = 0x03;
pub const TX_TRANSFER: u8 = 0x04;
pub const TX_FEE: u8 = 0x05;

#[derive(Debug, Clone, PartialEq, Eq, Canon)]
pub struct Transfer {
    pub from: Option<PublicKey>,
    pub to: Option<PublicKey>,
    pub amount: u64,
    pub timestamp: u64,
}
