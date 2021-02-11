// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::Canon;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::PublicKey;
use phoenix_core::{Message, Note};

#[derive(Debug, Clone, Canon)]
pub enum Call {
    None,

    SendToContractTransparent {
        address: BlsScalar,
        value: u64,
        value_commitment: JubJubAffine,
        pk: JubJubAffine,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparent {
        address: BlsScalar,
        note: Note,
    },

    SendToContractObfuscated {
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        crossover_commitment: JubJubAffine,
        crossover_pk: JubJubAffine,
        spend_proof: Vec<u8>,
    },

    WithdrawFromObfuscated {
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    },
}
