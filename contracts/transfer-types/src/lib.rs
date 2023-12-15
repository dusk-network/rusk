// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions with Dusk's transfer contract.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_pki::StealthAddress;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Module Id
pub type ModuleId = [u8; 32];

/// Send value to a contract transparently.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Stct {
    /// Module to send the value to.
    pub module: ModuleId,
    /// The value to send to the contract.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Vec<u8>,
}

/// Withdraw value from a contract transparently.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Wfct {
    ///     The value to withdraw
    pub value: u64,
    /// The note to withdraw transparently to
    pub note: Vec<u8>,
    /// A proof of the `WFCT` circuit.
    pub proof: Vec<u8>,
}

/// Mint value to a stealth address.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Mint {
    /// The address to mint to.
    pub address: StealthAddress,
    /// The value to mint to the address.
    pub value: u64,
    /// A nonce to prevent replay.
    pub nonce: BlsScalar,
}
