// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;

use std::ops::Deref;

use canonical_derive::Canon;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use dusk_wallet_core::Transaction;
use rusk_abi::hash::Hasher;
use rusk_schema::{TX_TYPE_COINBASE, TX_TYPE_TRANSFER, TX_VERSION};
use rusk_vm::GasMeter;

/// The payload for a transfer transaction.
///
/// Transfer transactions are the main type of transaction in the network.
/// They can be used to transfer funds, call contracts, and even both at the
/// same time.
#[derive(Debug, Clone, Canon)]
pub struct TransferPayload(dusk_wallet_core::Transaction);

impl Deref for TransferPayload {
    type Target = dusk_wallet_core::Transaction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TransferPayload {
    pub fn into_inner(self) -> Transaction {
        self.0
    }

    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        Ok(Self(dusk_wallet_core::Transaction::from_slice(buf)?))
    }
}

impl From<dusk_wallet_core::Transaction> for TransferPayload {
    fn from(tx: Transaction) -> Self {
        Self(tx)
    }
}

impl From<TransferPayload> for rusk_schema::Transaction {
    fn from(tx: TransferPayload) -> Self {
        let payload = tx.to_var_bytes();

        rusk_schema::Transaction {
            version: TX_VERSION,
            r#type: TX_TYPE_TRANSFER,
            payload,
        }
    }
}

/// The payload of a coinbase transaction.
///
/// Coinbase transactions are meant to award the block generator with the Dusk
/// spent in a block. They're not processed through the virtual machine. Instead
/// they are used to directly mutate the stake contract, incrementing the reward
/// for the given generator.
pub struct CoinbasePayload {
    pub block_height: u64,
    pub generator: PublicKey,
}

impl Serializable<104> for CoinbasePayload {
    type Error = BytesError;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let reader = &mut &buf[..];

        let block_height = u64::from_reader(reader)?;
        let generator = PublicKey::from_reader(reader)?;

        Ok(Self {
            block_height,
            generator,
        })
    }

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[0..8].copy_from_slice(&self.block_height.to_bytes());
        buf[8..104].copy_from_slice(&self.generator.to_bytes());

        buf
    }
}

impl From<CoinbasePayload> for rusk_schema::Transaction {
    fn from(coinbase: CoinbasePayload) -> Self {
        let payload = coinbase.to_bytes().to_vec();

        rusk_schema::Transaction {
            version: TX_VERSION,
            r#type: TX_TYPE_COINBASE,
            payload,
        }
    }
}

impl From<CoinbasePayload> for rusk_schema::ExecutedTransaction {
    fn from(coinbase: CoinbasePayload) -> Self {
        let payload = coinbase.to_bytes().to_vec();
        let tx_hash = Hasher::digest(&payload).to_bytes().to_vec();

        rusk_schema::ExecutedTransaction {
            tx: Some(coinbase.into()),
            tx_hash,
            gas_spent: 0,
            error: None,
        }
    }
}

pub struct SpentTransaction(
    pub TransferPayload,
    pub GasMeter,
    pub Option<Error>,
);

impl SpentTransaction {
    pub fn into_inner(self) -> (TransferPayload, GasMeter, Option<Error>) {
        (self.0, self.1, self.2)
    }
}

impl From<SpentTransaction> for rusk_schema::ExecutedTransaction {
    fn from(spent_tx: SpentTransaction) -> Self {
        let (transaction, gas_meter, error) = spent_tx.into_inner();
        let tx_hash = transaction.hash().to_bytes().to_vec();
        let gas_spent = gas_meter.spent();

        let error = error.map(|e| e.into());

        rusk_schema::ExecutedTransaction {
            error,
            tx: Some(transaction.into()),
            tx_hash,
            gas_spent,
        }
    }
}
