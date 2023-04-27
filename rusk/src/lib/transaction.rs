// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::Deref;

use dusk_bytes::{Error as BytesError, Serializable};
use phoenix_core::Transaction;
use rusk_abi::ModuleError;

use rusk_schema::executed_transaction::error::Code;
use rusk_schema::executed_transaction::Error;
use rusk_schema::{TX_TYPE_TRANSFER, TX_VERSION};

/// The payload for a transfer transaction.
///
/// Transfer transactions are the main type of transaction in the network.
/// They can be used to transfer funds, call contracts, and even both at the
/// same time.
#[derive(Debug, Clone)]
pub struct TransferPayload(Transaction);

impl Deref for TransferPayload {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TransferPayload {
    pub fn into_inner(self) -> Transaction {
        self.0
    }

    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        Ok(Self(Transaction::from_slice(buf)?))
    }
}

impl From<Transaction> for TransferPayload {
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

pub struct SpentTransaction(pub Transaction, pub u64, pub Option<ModuleError>);

impl SpentTransaction {
    pub fn into_inner(self) -> (Transaction, u64, Option<ModuleError>) {
        (self.0, self.1, self.2)
    }
}

impl From<SpentTransaction> for rusk_schema::ExecutedTransaction {
    fn from(spent_tx: SpentTransaction) -> Self {
        let (transaction, gas_spent, error) = spent_tx.into_inner();

        let tx_hash_input_bytes = transaction.to_hash_input_bytes();
        let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

        let error = error.map(|e| match e {
            ModuleError::Panic => Error {
                code: Code::ContractPanic.into(),
                contract_id: rusk_abi::transfer_module().to_bytes().to_vec(),
                data: String::from(""),
            },
            ModuleError::OutOfGas => Error {
                code: Code::OutOfGas.into(),
                contract_id: rusk_abi::transfer_module().to_bytes().to_vec(),
                data: String::from(""),
            },
            ModuleError::Other(_) => Error {
                code: Code::Other.into(),
                contract_id: rusk_abi::transfer_module().to_bytes().to_vec(),
                data: String::from(""),
            },
        });

        rusk_schema::ExecutedTransaction {
            error,
            tx: Some(transaction.into()),
            tx_hash: tx_hash.to_bytes().to_vec(),
            gas_spent,
        }
    }
}
