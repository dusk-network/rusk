// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! ![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-rusk-blueviolet?logo=github)](https://github.com/dusk-network/rusk)
//! [![Documentation](https://img.shields.io/badge/docs-rusk--abi-blue?logo=rust)](https://docs.rs/rusk-abi/)

//! # Rusk ABI
//!
//! The ABI to develop Rusk's specific Contracts

#![warn(missing_docs)]
#![no_std]
#![deny(clippy::all)]

use canonical::Canon;
use canonical_derive::Canon;
use dusk_abi::{ContractId, Module};
use dusk_pki::PublicSpendKey;
mod public_input;
pub use public_input::PublicInput;

/// Module that exports the ABI for Rusk's Contracts
///
/// Any proof to be verified with this module should use `b"dusk-network` as
/// transcript initialization
#[allow(dead_code)]
pub struct RuskModule {
    #[cfg(not(target_arch = "wasm32"))]
    pp: &'static dusk_plonk::prelude::PublicParameters,
}

impl RuskModule {
    #[doc(hidden)]
    pub const POSEIDON_HASH: u8 = 0;
    #[doc(hidden)]
    pub const VERIFY_PROOF: u8 = 1;
    #[doc(hidden)]
    pub const VERIFY_SCHNORR_SIGN: u8 = 2;
}

impl Module for RuskModule {
    fn id() -> ContractId {
        ContractId::reserved(77)
    }
}

/// Enum that represents all possible payment info configs
#[derive(Canon, Clone)]
pub enum PaymentInfo {
    /// Only Transparent Notes are accepted
    Transparent(Option<PublicSpendKey>),
    /// Only Obfuscated Notes are accepted
    Obfuscated(Option<PublicSpendKey>),
    /// Notes of any type are accepted
    Any(Option<PublicSpendKey>),
}

/// Common QueryId used for Payment info retrival.
pub const PAYMENT_INFO: u8 = 100;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[doc(hidden)]
        pub mod hosted;
        pub use hosted::*;
    } else {
        #[doc(hidden)]
        pub mod host;
        pub use host::*;
    }
}
