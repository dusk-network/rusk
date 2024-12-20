// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//![doc = include_str!("../README.md")]

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
    // the `unused_crate_dependencies` lint complains for dev-dependencies that
    // are only used in integration tests, so adding this work-around here
    use ff as _;
    use once_cell as _;
    use rand as _;
}
