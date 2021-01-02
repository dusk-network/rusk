// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract, Counter};
use canonical_host::{MemStore, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature, APK};
use dusk_plonk::prelude::*;

type TransactionIndex = u16;

impl Contract<MemStore> {
    pub fn stake(
        block_height: u64,
        value: u64,
        public_key: APK,
        /* spending_proof: Proof,
         * pub_inputs_len: u8,
         * pub_inputs: [[u8; PublicInput::serialized_size()]; 1], */
    ) -> Transaction<
        (
            TransactionIndex,
            u64,
            u64,
            APK, /* , Proof, u8, [[u8; 33]; 1] */
        ),
        (Counter, bool),
    > {
        Transaction::new((ops::STAKE, block_height, value, public_key))
    }

    pub fn extend_stake(
        w_i: Counter,
        public_key: APK,
        sig: Signature,
    ) -> Transaction<(TransactionIndex, Counter, APK, Signature), bool> {
        Transaction::new((ops::EXTEND_STAKE, w_i, public_key, sig))
    }

    pub fn withdraw_stake(
        block_height: u64,
        w_i: Counter,
        public_key: APK,
        sig: Signature,
        /* note */
    ) -> Transaction<
        (
            TransactionIndex,
            u64,
            Counter,
            APK,
            Signature, /* note */
        ),
        bool,
    > {
        Transaction::new((
            ops::WITHDRAW_STAKE,
            block_height,
            w_i,
            public_key,
            sig,
            /* note */
        ))
    }

    pub fn slash(
        public_key: APK,
        round: u64,
        step: u8,
        message_1: BlsScalar,
        message_2: BlsScalar,
        signature_1: Signature,
        signature_2: Signature,
        /* note */
    ) -> Transaction<
        (
            TransactionIndex,
            APK,
            u64,
            u8,
            BlsScalar,
            BlsScalar,
            Signature,
            Signature,
        ),
        bool,
    > {
        Transaction::new((
            ops::SLASH,
            public_key,
            round,
            step,
            message_1,
            message_2,
            signature_1,
            signature_2,
        ))
    }
}
