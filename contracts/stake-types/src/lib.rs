// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions with Dusk's stake contract.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

extern crate alloc;
use alloc::vec::Vec;

mod sig;
mod stake;

pub use sig::*;
pub use stake::*;

use dusk_bls12_381_sign::{Signature, APK};
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use phoenix_core::Note;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Stake a value on the stake contract.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Stake {
    /// Public key to which the stake will belong.
    pub public_key: APK,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// Value to stake.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Proof,
}
impl Stake {
    /// Serializes stake in a vector of bytes
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 8192>(self)
            .expect("Serializing stake transaction should succeed")
            .to_vec()
    }
}

/// Unstake a value from the stake contract.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Unstake {
    /// Public key to unstake.
    pub public_key: APK,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// Note to withdraw to.
    pub note: Note,
    /// A proof of the `WFCT` circuit.
    pub proof: Proof,
}

/// Withdraw the accumulated reward.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    /// Public key to withdraw the rewards.
    pub public_key: APK,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// The address to mint to.
    pub address: StealthAddress,
    /// A nonce to prevent replay.
    pub nonce: BlsScalar,
}

/// Allow a public key to stake.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Allow {
    /// The public key to allow staking to.
    pub public_key: APK,
    /// The "owner" of the smart contract.
    pub owner: APK,
    /// Signature of the `owner` key.
    pub signature: Signature,
}
