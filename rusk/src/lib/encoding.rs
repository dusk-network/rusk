// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]
use crate::error::Error;
use crate::services::state::SpentTransaction;
use crate::transaction::{Transaction, TransactionPayload};
use core::convert::TryFrom;
use dusk_bytes::Serializable;
use rusk_schema::{TX_TYPE_TRANSFER, TX_VERSION};

use rusk_vm::VMError;
use std::convert::TryInto;
use tonic::{Code, Status};

impl From<&Transaction> for rusk_schema::Transaction {
    fn from(value: &Transaction) -> Self {
        let buf = value.payload.to_bytes();

        rusk_schema::Transaction {
            version: value.version.into(),
            r#type: value.tx_type.into(),
            payload: buf,
        }
    }
}

impl TryFrom<&mut Transaction> for rusk_schema::Transaction {
    type Error = Status;

    fn try_from(value: &mut Transaction) -> Result<Self, Status> {
        Ok(rusk_schema::Transaction::from(&*value))
    }
}

impl TryFrom<&mut rusk_schema::Transaction> for Transaction {
    type Error = Status;

    fn try_from(
        value: &mut rusk_schema::Transaction,
    ) -> Result<Transaction, Status> {
        let payload = TransactionPayload::from_bytes(value.payload.as_slice())
            .map_err(|e| {
                Status::new(Code::InvalidArgument, format!("{}", e))
            })?;

        Ok(Transaction {
            version: value
                .version
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            tx_type: value
                .r#type
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            payload,
        })
    }
}

impl From<Error> for rusk_schema::executed_transaction::Error {
    fn from(err: Error) -> Self {
        use rusk_schema::executed_transaction::error::Code;

        let (code, contract_id, data) = match err {
            Error::Vm(e) => match e {
                VMError::UnknownContract(id) => {
                    (Code::UnknownContract, id, format!("{}", e))
                }
                VMError::ContractPanic(id, data) => {
                    (Code::ContractPanic, id, data)
                }
                VMError::OutOfGas => (
                    Code::OutOfGas,
                    rusk_abi::transfer_contract(),
                    format!("{}", e),
                ),
                _ => (
                    Code::Other,
                    rusk_abi::transfer_contract(),
                    format!("{}", e),
                ),
            },
            _ => (
                Code::Other,
                rusk_abi::transfer_contract(),
                format!("{}", err),
            ),
        };

        Self {
            code: code.into(),
            contract_id: contract_id.as_bytes().to_vec(),
            data,
        }
    }
}

impl From<SpentTransaction> for rusk_schema::ExecutedTransaction {
    fn from(spent_tx: SpentTransaction) -> Self {
        let (transaction, gas_meter, error) = spent_tx.into_inner();
        let tx_hash = transaction.hash().to_bytes().to_vec();
        let gas_spent = gas_meter.spent();

        let error = error.map(|e| e.into());

        let tx = Some(rusk_schema::Transaction {
            version: TX_VERSION,
            r#type: TX_TYPE_TRANSFER,
            payload: transaction.to_var_bytes(),
        });

        rusk_schema::ExecutedTransaction {
            error,
            tx,
            tx_hash,
            gas_spent,
        }
    }
}
