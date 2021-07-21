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

#[allow(dead_code)]
pub(crate) mod genesis {
    use dusk_bytes::hex;

    /// Transfer Contract Address
    pub const TRANSFER_ADDRESS: [u8; 32] = hex(
        b"5a1cd7c445cfd23bdf300bef21aa70ea7995417274c3ec62f4e3ad1a2421c45c",
    );

    /// Stake Contract Adddress
    pub const STAKE_ADDRESS: [u8; 32] = hex(
        b"02db0c1b7401be50af6c7fe5e1bd2a5656464efc0c272528a790fb17930d9dfb",
    );
}

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
