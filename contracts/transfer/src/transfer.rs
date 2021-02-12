// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use phoenix_core::{Message, Note};

mod tree;
use tree::Tree;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Canon)]
pub struct PublicKeyBytes([u8; PublicKey::SIZE]);

impl From<PublicKey> for PublicKeyBytes {
    fn from(pk: PublicKey) -> Self {
        Self(pk.to_bytes())
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct Transfer<S: Store> {
    pub(crate) notes: Tree<S>,
    pub(crate) notes_mapping: Map<u64, Vec<Note>, S>,
    pub(crate) nullifiers: Map<BlsScalar, (), S>,
    pub(crate) roots: Map<BlsScalar, (), S>,
    pub(crate) balance: Map<BlsScalar, u64, S>,
    pub(crate) message_mapping:
        Map<BlsScalar, Map<PublicKeyBytes, Message, S>, S>,
    pub(crate) message_mapping_set:
        Map<BlsScalar, (PublicKey, JubJubAffine), S>,
}

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
