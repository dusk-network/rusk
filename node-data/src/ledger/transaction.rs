// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable as DuskSerializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::phoenix::Transaction as PhoenixTransaction;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use serde::Serialize;
use sha3::Digest;

use crate::Serializable;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u32,
    pub r#type: u32,
    pub inner: ProtocolTransaction,
    pub(crate) size: Option<usize>,
}

impl Transaction {
    pub fn size(&self) -> usize {
        match self.size {
            Some(size) => size,
            None => {
                let mut buf = vec![];
                self.write(&mut buf).expect("write to vec should not fail");
                buf.len()
            }
        }
    }
}

impl From<ProtocolTransaction> for Transaction {
    fn from(value: ProtocolTransaction) -> Self {
        Self {
            inner: value,
            r#type: 1,
            version: 1,
            size: None,
        }
    }
}

/// A spent transaction is a transaction that has been included in a block and
/// was executed.
#[derive(Debug, Clone, Serialize)]
pub struct SpentTransaction {
    /// The transaction that was executed.
    pub inner: Transaction,
    /// The height of the block in which the transaction was included.
    pub block_height: u64,
    /// The amount of gas that was spent during the execution of the
    /// transaction.
    pub gas_spent: u64,
    /// An optional error message if the transaction execution yielded an
    /// error.
    pub err: Option<String>,
}

impl SpentTransaction {
    /// Returns the underlying public transaction, if it is one. Otherwise,
    /// returns `None`.
    pub fn public(&self) -> Option<&MoonlightTransaction> {
        match &self.inner.inner {
            ProtocolTransaction::Moonlight(public_tx) => Some(public_tx),
            _ => None,
        }
    }

    /// Returns the underlying shielded transaction, if it is one. Otherwise,
    /// returns `None`.
    pub fn shielded(&self) -> Option<&PhoenixTransaction> {
        match &self.inner.inner {
            ProtocolTransaction::Phoenix(shielded_tx) => Some(shielded_tx),
            _ => None,
        }
    }
}

impl Transaction {
    /// Computes the hash digest of the entire transaction data.
    ///
    /// This method returns the Sha3 256 digest of the entire
    /// transaction in its serialized form
    ///
    /// The digest hash is currently only being used in the merkle tree.
    ///
    /// ### Returns
    /// An array of 32 bytes representing the hash of the transaction.
    pub fn digest(&self) -> [u8; 32] {
        sha3::Sha3_256::digest(self.inner.to_var_bytes()).into()
    }

    /// Computes the transaction ID.
    ///
    /// The transaction ID is a unique identifier for the transaction.
    /// Unlike the [`digest()`](#method.digest) method, which is computed over
    /// the entire transaction, the transaction ID is derived from specific
    /// fields of the transaction and serves as a unique identifier of the
    /// transaction itself.
    ///
    /// ### Returns
    /// An array of 32 bytes representing the transaction ID.
    pub fn id(&self) -> [u8; 32] {
        self.inner.hash().to_bytes()
    }

    pub fn gas_price(&self) -> u64 {
        self.inner.gas_price()
    }

    pub fn to_spend_ids(&self) -> Vec<SpendingId> {
        match &self.inner {
            ProtocolTransaction::Phoenix(p) => p
                .nullifiers()
                .iter()
                .map(|n| SpendingId::Nullifier(n.to_bytes()))
                .collect(),
            ProtocolTransaction::Moonlight(m) => {
                vec![SpendingId::AccountNonce(*m.sender(), m.nonce())]
            }
        }
    }

    pub fn next_spending_id(&self) -> Option<SpendingId> {
        match &self.inner {
            ProtocolTransaction::Phoenix(_) => None,
            ProtocolTransaction::Moonlight(m) => {
                Some(SpendingId::AccountNonce(*m.sender(), m.nonce() + 1))
            }
        }
    }
}

impl PartialEq<Self> for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type
            && self.version == other.version
            && self.id() == other.id()
    }
}

impl Eq for Transaction {}

impl PartialEq<Self> for SpentTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.gas_spent == other.gas_spent
    }
}

impl Eq for SpentTransaction {}

pub enum SpendingId {
    Nullifier([u8; 32]),
    AccountNonce(AccountPublicKey, u64),
}

impl SpendingId {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SpendingId::Nullifier(n) => n.to_vec(),
            SpendingId::AccountNonce(account, nonce) => {
                let mut id = account.to_bytes().to_vec();
                id.extend_from_slice(&nonce.to_le_bytes());
                id
            }
        }
    }

    pub fn next(&self) -> Option<SpendingId> {
        match self {
            SpendingId::Nullifier(_) => None,
            SpendingId::AccountNonce(account, nonce) => {
                Some(SpendingId::AccountNonce(*account, nonce + 1))
            }
        }
    }
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use dusk_core::transfer::data::{ContractCall, TransactionData};
    use dusk_core::transfer::phoenix::{
        Fee, Note, Payload as PhoenixPayload, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
        TxSkeleton,
    };
    use dusk_core::{BlsScalar, JubJubScalar};
    use rand::Rng;

    use super::*;
    use crate::ledger::Dummy;

    impl<T> Dummy<T> for Transaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            gen_dummy_tx(1_000_000)
        }
    }

    impl<T> Dummy<T> for SpentTransaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            let tx = gen_dummy_tx(1_000_000);
            SpentTransaction {
                inner: tx,
                block_height: 0,
                gas_spent: 3,
                err: Some("error".to_string()),
            }
        }
    }

    /// Generates a decodable transaction from a fixed blob with a specified
    /// gas price.
    pub fn gen_dummy_tx(gas_price: u64) -> Transaction {
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::new(
            JubJubScalar::from(42u64),
            JubJubScalar::from(42u64),
        ));
        let gas_limit = 1;

        let fee = Fee::deterministic(
            &JubJubScalar::from(5u64),
            &pk,
            gas_limit,
            gas_price,
            &[JubJubScalar::from(9u64), JubJubScalar::from(10u64)],
        );

        let tx_skeleton = TxSkeleton {
            root: BlsScalar::from(12345u64),
            nullifiers: vec![
                BlsScalar::from(1u64),
                BlsScalar::from(2u64),
                BlsScalar::from(3u64),
            ],
            outputs: [Note::empty(), Note::empty()],
            max_fee: gas_price * gas_limit,
            deposit: 0,
        };

        let contract_call =
            ContractCall::new([21; 32], "some_method", &()).unwrap();

        let payload = PhoenixPayload {
            chain_id: 0xFA,
            tx_skeleton,
            fee,
            data: Some(TransactionData::Call(contract_call)),
        };
        let proof = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        let tx: ProtocolTransaction =
            PhoenixTransaction::from_payload_and_proof(payload, proof).into();

        tx.into()
    }
}
