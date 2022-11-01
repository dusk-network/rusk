// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_pki::StealthAddress;
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Message, Note};
use piecrust_uplink::ModuleId;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// A phoenix transaction.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transaction {
    /// The root of the transfer tree on top of which this transaction is
    /// based.
    pub anchor: BlsScalar,
    /// The nullifiers of the notes this transaction spends.
    pub nullifiers: Vec<BlsScalar>,
    /// The output notes of this transaction.
    pub outputs: Vec<Note>,
    /// Describes the fee to be paid for this transaction.
    pub fee: Fee,
    /// A crossover is used to transferring funds to a contract - i.e. in
    /// [`STCT`] and [`STCO`].
    pub crossover: Option<Crossover>,
    /// A proof of the `Execute` circuit for this transaction.
    pub proof: Proof,
    /// A call to a contract. The `Vec<u8>` must be an `rkyv`ed representation
    /// of the data the contract expects.
    pub call: Option<(ModuleId, Vec<u8>)>,
}

/// Send value to a contract transparently.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct STCT {
    /// Module to send the value to.
    pub module_id: ModuleId,
    /// The value to send to the contract.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Proof,
}

/// Withdraw value from a contract transparently.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WFCT {
    /// The value to withdraw
    pub value: u64,
    /// The note to withdraw transparently to
    pub note: Note,
    /// A proof of the `WFCT` circuit.
    pub proof: Proof,
}

/// Send value to a contract anonymously.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct STCO {
    /// Module to send the value to.
    pub module_id: ModuleId,
    /// Message containing the value commitment.
    pub message: Message,
    /// The stealth address of the message.
    pub message_address: StealthAddress,
    /// Proof of the `STCO` circuit.
    pub proof: Proof,
}

/// Withdraw value from a contract anonymously.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WFCO {
    /// Message containing the value commitment.
    pub message: Message,
    /// The stealth address of the message.
    pub message_address: StealthAddress,
    /// Message containing commitment on the change value.
    pub change: Message,
    /// The stealth address of the change message.
    pub change_address: StealthAddress,
    /// The note to withdraw to.
    pub output: Note,
    /// Proof of the `WFCO` circuit.
    pub proof: Proof,
}

/// Withdraw value from the calling contract to another contract.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WFCTC {
    /// The contract to transfer value to.
    pub to: ModuleId,
    /// The value to transfer.
    pub value: u64,
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
