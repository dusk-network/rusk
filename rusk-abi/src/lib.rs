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
//! The ABI to develop Dusk Network smart contracts

#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

mod host;
pub use host::{
    hash, new_ephemeral_vm, new_genesis_session, new_session, new_vm,
    poseidon_hash, verify_bls, verify_bls_multisig, verify_plonk,
    verify_schnorr,
};
pub use piecrust::{
    CallReceipt, CallTree, CallTreeElem, ContractData, Error as PiecrustError,
    PageOpening, Session, VM,
};

#[cfg(test)]
mod tests {
    // rust doesn't allow for optional dev-dependencies so we need to add this
    // work-around to satisfy the `unused_crate_dependencies` lint
    use ff as _;
    use once_cell as _;
    use rand as _;
}
