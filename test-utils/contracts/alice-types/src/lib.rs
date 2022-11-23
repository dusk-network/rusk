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

use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use phoenix_core::{Message, Note};
use rusk_abi::ModuleId;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Withdraw a value in a transparent mode.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    /// Value to withdraw.
    pub value: u64,
    /// Note to withdraw to.
    pub note: Note,
    /// Proof of the `STCT` circuit.
    pub proof: Proof,
}

/// Withdraw a value in an obfuscated mode.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawObfuscated {
    /// Message containing the value commitment.
    pub message: Message,
    /// The address to which withdraw the value.
    pub message_address: StealthAddress,
    /// Message containing the change commitment.
    pub change: Message,
    /// The address to which withdraw the change.
    pub change_address: StealthAddress,
    /// Note to withdraw to.
    pub output: Note,
    /// Proof of the `STCO` circuit.
    pub proof: Proof,
}

/// Withdraw a value into a contract.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawToContract {
    /// The module to transfer value to.
    pub module: ModuleId,
    /// The value to transfer.
    pub value: u64,
}
