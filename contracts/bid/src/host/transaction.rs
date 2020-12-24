// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract};
use canonical_host::{MemStore, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use phoenix_core::Note;
use poseidon252::cipher::PoseidonCipher;
use schnorr::single_key::{PublicKey, Signature};

type TransactionIndex = u16;

impl Contract<MemStore> {
    pub fn bid(
        commitment: JubJubAffine,
        hashed_secret: BlsScalar,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
        stealth_address: StealthAddress,
        block_height: u64,
        correctness_proof: Proof,
        spending_proof: Proof,
        pub_inputs_len: u8,
        pub_inputs: [[u8; PublicInput::serialized_size()]; 1],
    ) -> Transaction<
        (
            TransactionIndex,
            JubJubAffine,
            BlsScalar,
            BlsScalar,
            PoseidonCipher,
            StealthAddress,
            u64,
            Proof,
            Proof,
            u8,
            [[u8; 33]; 1],
        ),
        (bool, u64),
    > {
        Transaction::new((
            ops::BID,
            commitment,
            hashed_secret,
            nonce,
            encrypted_data,
            stealth_address,
            block_height,
            correctness_proof,
            spending_proof,
            pub_inputs_len,
            pub_inputs,
        ))
    }

    pub fn withdraw(
        signature: Signature,
        pub_key: PublicKey,
        note: Note,
        spending_proof: Proof,
        block_height: u64,
    ) -> Transaction<
        (TransactionIndex, Signature, PublicKey, Note, Proof, u64),
        bool,
    > {
        Transaction::new((
            ops::WITHDRAW,
            signature,
            pub_key,
            note,
            spending_proof,
            block_height,
        ))
    }

    pub fn extend_bid(
        signature: Signature,
        pub_key: PublicKey,
    ) -> Transaction<(TransactionIndex, Signature, PublicKey), bool> {
        Transaction::new((ops::EXTEND_BID, signature, pub_key))
    }
}
