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

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey, Signature};
use dusk_pki::StealthAddress;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Stake a value on the stake contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Stake {
    /// Public key to which the stake will belong.
    pub public_key: PublicKey,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// Value to stake.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Vec<u8>,
}

/// Unstake a value from the stake contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Unstake {
    /// Public key to unstake.
    pub public_key: PublicKey,
    /// Signature belonging to the given public key.
    pub signature: Signature,
    /// Note to withdraw to.
    pub note: Vec<u8>, // todo: not sure it will stay as Vec
    /// A proof of the `WFCT` circuit.
    pub proof: Vec<u8>,
}

/// Withdraw the accumulated reward.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    /// Public key to withdraw the rewards.
    pub public_key: PublicKey,
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
    pub public_key: PublicKey,
    /// The "owner" of the smart contract.
    pub owner: PublicKey,
    /// Signature of the `owner` key.
    pub signature: Signature,
}
