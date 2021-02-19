// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::{ByteSource, Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_abi::Transaction;
use phoenix_core::Note;

#[derive(Debug, Clone, Canon)]
pub enum Call {
    Stake {
        value: u64,
        public_key: APK,
        spending_proof: Vec<u8>,
    },

    ExtendStake {
        w_i: Counter,
        public_key: APK,
        sig: Signature,
    },

    WithdrawStake {
        w_i: Counter,
        public_key: APK,
        sig: Signature,
        note: Note,
    },

    Slash {
        public_key: APK,
        round: u64,
        step: u8,
        message_1: BlsScalar,
        message_2: BlsScalar,
        signature_1: Signature,
        signature_2: Signature,
        note: Note,
    },
}

impl Call {
    fn to_transaction<S>(&self) -> result<Transaction, S::Error>
    where
        S: Store,
    {
        // FIXME BytesSink should not require `store`
        // https://github.com/dusk-network/canonical/issues/71
        let store: &S =
            unsafe { (&() as *const ()).cast::<S>().as_ref().unwrap() };

        Transaction::from_canon(self, store)
    }

    pub fn stake<S>(
        value: u64,
        public_key: APK,
        spending_proof: Vec<u8>,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::Stake {
            value,
            public_key,
            spending_proof,
        };

        call.to_transaction::<S>()
    }

    pub fn extend_stake<S>(
        w_i: Counter,
        public_key: APK,
        sig: Signature,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::ExtendStake {
            w_i,
            public_key,
            sig,
        };

        call.to_transaction::<S>()
    }

    pub fn withdraw_stake<S>(
        w_i: Counter,
        public_key: APK,
        sig: Signature,
        note: Note,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::WithdrawStake {
            w_i,
            public_key,
            sig,
            note,
        };

        call.to_transaction::<S>()
    }

    pub fn slash<S>(
        public_key: APK,
        round: u64,
        step: u8,
        message_1: BlsScalar,
        message_2: BlsScalar,
        signature_1: Signature,
        signature_2: Signature,
        note: Note,
    ) -> Result<Transaction, S::Error>
    where
        S: Store,
    {
        let call = Self::Slash {
            public_key,
            round,
            step,
            message_1,
            message_2,
            signature_1,
            signature_2,
            note,
        };

        call.to_transaction::<S>()
    }
}
