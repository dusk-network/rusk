// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! ![Build Status](https://github.com/dusk-network/rusk-abi/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-rusk--abi-blueviolet?logo=github)](https://github.com/dusk-network/rusk-abi)
//! [![Documentation](https://img.shields.io/badge/docs-rusk--abi-blue?logo=rust)](https://docs.rs/rusk-abi/)

//! # Rusk ABI
//!
//! The ABI to develop Rusk's specific Contracts
#![warn(missing_docs)]
#![no_std]

use dusk_abi::{ContractId, Module};

/// Module that exports the ABI for Rusk's Contracts
#[allow(dead_code)]
pub struct RuskModule<S> {
    store: S,
}

impl<S> RuskModule<S> {
    #[doc(hidden)]
    pub const POSEIDON_HASH: u8 = 0;
    #[doc(hidden)]
    pub const VERIFY_SCHNORR_SIGN: u8 = 2;
}

impl<S> Module for RuskModule<S> {
    fn id() -> ContractId {
        ContractId::reserved(77)
    }
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
