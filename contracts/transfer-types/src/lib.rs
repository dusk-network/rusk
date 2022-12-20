// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions with Dusk's transfer contract.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

mod circuits;
pub use circuits::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use core::borrow::Borrow;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::StealthAddress;
use dusk_plonk::proof_system::Proof;
use dusk_poseidon::tree::PoseidonLeaf;
use nstack::annotation::Keyed;
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::ModuleId;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// The depth of the transfer tree.
pub const TRANSFER_TREE_DEPTH: usize = 17;

/// A leaf of the transfer tree.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TreeLeaf {
    /// The height of the block when the note was inserted in the tree.
    pub block_height: u64,
    /// The note inserted in the tree.
    pub note: Note,
}

impl PoseidonLeaf for TreeLeaf {
    fn poseidon_hash(&self) -> BlsScalar {
        rusk_abi::poseidon_hash(self.note.hash_inputs().into())
    }

    fn pos(&self) -> &u64 {
        self.note.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.note.set_pos(pos);
    }
}

impl Keyed<u64> for TreeLeaf {
    fn key(&self) -> &u64 {
        &self.block_height
    }
}

impl AsRef<Note> for TreeLeaf {
    fn as_ref(&self) -> &Note {
        &self.note
    }
}

impl Borrow<u64> for TreeLeaf {
    fn borrow(&self) -> &u64 {
        self.note.pos()
    }
}

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
    /// [`Stct`] and [`Stco`].
    pub crossover: Option<Crossover>,
    /// A proof of the `Execute` circuit for this transaction.
    pub proof: Proof,
    /// A call to a contract. The `Vec<u8>` must be an `rkyv`ed representation
    /// of the data the contract expects, and the `String` the name of the
    /// function to call.
    pub call: Option<(ModuleId, String, Vec<u8>)>,
}

impl Transaction {
    /// Returns the hash of this transaction.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        Self::hash_from_components(
            &self.nullifiers,
            &self.outputs,
            &self.anchor,
            &self.fee,
            &self.crossover,
            &self.call,
        )
    }

    /// Hash a transaction from its components.
    #[must_use]
    pub fn hash_from_components(
        nullifiers: &[BlsScalar],
        outputs: &[Note],
        anchor: &BlsScalar,
        fee: &Fee,
        crossover: &Option<Crossover>,
        call: &Option<(ModuleId, String, Vec<u8>)>,
    ) -> BlsScalar {
        let mut bytes = Vec::new();

        for nullifier in nullifiers {
            bytes.extend(nullifier.to_bytes());
        }
        for note in outputs {
            bytes.extend(note.to_bytes());
        }

        bytes.extend(anchor.to_bytes());
        bytes.extend(fee.to_bytes());

        if let Some(crossover) = crossover {
            bytes.extend(crossover.to_bytes());
        }

        if let Some((module, fn_name, call_data)) = call {
            bytes.extend(module.as_bytes());
            bytes.extend(fn_name.as_bytes());
            bytes.extend(call_data);
        }

        rusk_abi::hash(bytes)
    }
}

/// Send value to a contract transparently.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Stct {
    /// Module to send the value to.
    pub module: ModuleId,
    /// The value to send to the contract.
    pub value: u64,
    /// Proof of the `STCT` circuit.
    pub proof: Proof,
}

/// Withdraw value from a contract transparently.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Wfct {
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
pub struct Stco {
    /// Module to send the value to.
    pub module: ModuleId,
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
pub struct Wfco {
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
pub struct Wfctc {
    /// The contract to transfer value to.
    pub module: ModuleId,
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
