// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;

use dusk_bytes::Serializable as DuskSerializable;
use execution_core::signatures::bls;
use execution_core::transfer::Transaction as ProtocolTransaction;
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
    pub fn size(&self) -> io::Result<usize> {
        match self.size {
            Some(size) => Ok(size),
            None => {
                let mut buf = vec![];
                self.write(&mut buf)?;
                Ok(buf.len())
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

#[derive(Debug, Clone, Serialize)]
pub struct SpentTransaction {
    pub inner: Transaction,
    pub block_height: u64,
    pub gas_spent: u64,
    pub err: Option<String>,
}

impl Transaction {
    /// Computes the hash of the transaction.
    ///
    /// This method returns the hash of the entire
    /// transaction in its serialized form
    ///
    /// ### Returns
    /// An array of 32 bytes representing the hash of the transaction.
    pub fn hash(&self) -> [u8; 32] {
        sha3::Sha3_256::digest(self.inner.to_var_bytes()).into()
    }

    /// Computes the transaction ID.
    ///
    /// The transaction ID is a unique identifier for the transaction.
    /// Unlike the [`hash()`](#method.hash) method, which is computed over the
    /// entire transaction, the transaction ID is derived from specific
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
    AccountNonce(bls::PublicKey, u64),
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
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use execution_core::transfer::data::{ContractCall, TransactionData};
    use execution_core::transfer::phoenix::{
        Fee, Note, Payload as PhoenixPayload, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
        TxSkeleton,
    };
    use execution_core::{BlsScalar, JubJubScalar};
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
