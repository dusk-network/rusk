// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions which support subsidizing.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::vec::Vec;

mod sig;

pub use sig::*;

use dusk_bls12_381_sign::{PublicKey, Signature};

use rkyv::{Archive, Deserialize, Serialize};

/// Subsidy a contract with a value.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Subsidy {
    /// Public key to which the subsidy will belong.
    pub public_key: PublicKey,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// Value to subsidize.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Vec<u8>,
}
