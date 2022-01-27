// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod contract;
mod error;
mod stake;

pub use contract::StakeContract;
pub use error::Error;
pub use stake::Stake;

#[cfg(target_arch = "wasm32")]
mod wasm;

/// Epoch used for stake operations
pub const EPOCH: u64 = 2160;

/// Maturity of the stake
pub const MATURITY: u64 = 2 * EPOCH;

/// Validity of the stake
pub const VALIDITY: u64 = 56 * EPOCH;

/// The minimum amount of (micro)Dusk one can stake.
pub const MINIMUM_STAKE: u64 = 5_000_000_000;

pub const TX_STAKE: u8 = 0x00;
pub const TX_EXTEND: u8 = 0x01;
pub const TX_WITHDRAW: u8 = 0x02;
