// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_pki::StealthAddress;
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Message, Note};
use piecrust_uplink::ModuleId;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Execute {
    anchor: BlsScalar,
    nullifiers: Vec<BlsScalar>,
    fee: Fee,
    crossover: Option<Crossover>,
    notes: Vec<Note>,
    spend_proof: Proof,
    call: Option<(ModuleId, Vec<u8>)>,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SendToContractTransparent {
    module_id: ModuleId,
    value: u64,
    proof: Proof,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawFromContractTransparent {
    value: u64,
    note: Note,
    proof: Proof,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SendToContractObfuscated {
    module_id: ModuleId,
    message: Message,
    message_address: StealthAddress,
    proof: Proof,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawFromContractObfuscated {
    message: Message,
    message_address: StealthAddress,
    change: Message,
    change_address: StealthAddress,
    output: Note,
    proof: Proof,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawFromContractToContractTransparent {
    to: ModuleId,
    value: u64,
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Mint {
    address: StealthAddress,
    value: u64,
    nonce: BlsScalar,
}
