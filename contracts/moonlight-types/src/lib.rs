// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for transactions with Dusk's moonlight contract.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use bls12_381_bls::{PublicKey as BlsPublicKey, Signature as BlsSignature};
use bytecheck::CheckBytes;
use dusk_bytes::Serializable as _;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Account {
    pub nonce: u64,
    pub balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Deposit {
    pub address: BlsPublicKey,
    pub value: u64,
    pub proof: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Transfer {
    pub from_address: BlsPublicKey,
    pub nonce: u64,
    pub to_address: BlsPublicKey,
    pub value: u64,
    pub signature: BlsSignature,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Withdraw {
    pub address: BlsPublicKey,
    pub nonce: u64,
    pub value: u64,
    pub note: Vec<u8>,
    pub proof: Vec<u8>,
    pub signature: BlsSignature,
}

impl Transfer {
    pub fn to_signature_message(&self) -> Vec<u8> {
        let mut message = Vec::with_capacity(2 * BlsPublicKey::SIZE + 16);

        message.extend_from_slice(&self.from_address.to_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message.extend_from_slice(&self.to_address.to_bytes());
        message.extend_from_slice(&self.value.to_le_bytes());

        message
    }
}

impl Withdraw {
    pub fn to_signature_message(&self) -> Vec<u8> {
        let mut message =
            Vec::with_capacity(BlsPublicKey::SIZE + 16 + self.note.len() + self.proof.len());

        message.extend_from_slice(&self.address.to_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message.extend_from_slice(&self.value.to_le_bytes());
        message.extend_from_slice(&self.note);
        message.extend_from_slice(&self.proof);

        message
    }
}

///
/// Events

/// Event emitted after a mutation of the Moonlight contract state.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct MoonlightEvent {
    /// Active address of the operation.
    pub active_address: Option<BlsPublicKey>,
    /// Passive address that was affected by the operation.
    pub passive_address: Option<BlsPublicKey>,
    /// Mutation value.
    pub value: u64,
}
