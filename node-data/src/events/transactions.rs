// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;

use dusk_core::transfer::RefundAddress;

use super::*;
use crate::ledger::{Hash, SpentTransaction, Transaction};

/// Represents events related to transactions.
///
/// - `Removed(Hash)`
///
///     Indicates that a transaction has been removed from the mempool. The
///     `Hash` represents the unique identifier of the removed transaction.
///
///     This event is triggered when a transaction is removed from the mempool
///     or discarded from the mempool.
///
/// - `Included(&'t Transaction)`
///
///     A transaction has been included in the mempool.
///
/// - `Executed(&'t SpentTransaction)`
///
///     Denotes that a transaction has been executed into an accepted block.
///     Executed transactions also include failed transaction, as they have been
///     spent and were correctly executed according to any contract logic.
///     (including logic that triggers panics)
///
///     - A "successful" transaction: executed and the `err` field is `None`.
///     - A "failed" transaction: executed and the `err` field is `Some`.
#[derive(Clone, Debug)]
pub enum TransactionEvent<'t> {
    Removed(Hash),
    Included(&'t Transaction),
    Executed(&'t SpentTransaction),
}

impl EventSource for TransactionEvent<'_> {
    const COMPONENT: &'static str = "transactions";

    fn topic(&self) -> &'static str {
        match self {
            Self::Removed(_) => "removed",
            Self::Executed(_) => "executed",
            Self::Included(_) => "included",
        }
    }
    fn data(&self) -> Option<serde_json::Value> {
        match self {
            Self::Removed(_) => None,
            Self::Executed(t) => serde_json::to_value(t).ok(),
            Self::Included(t) => serde_json::to_value(t).ok(),
        }
    }
    fn entity(&self) -> String {
        let hash = match self {
            Self::Removed(hash) => *hash,
            Self::Executed(tx) => tx.inner.id(),
            Self::Included(tx) => tx.id(),
        };
        hex::encode(hash)
    }
}
use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use dusk_bytes::Serializable;
use dusk_core::transfer::Transaction as ProtocolTransaction;
use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for Transaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Transaction", 1)?;
        match &self.inner {
            ProtocolTransaction::Phoenix(p) => {
                state.serialize_field("type", "phoenix")?;

                let root = p.root().to_bytes();
                state.serialize_field("root", &hex::encode(root))?;

                let nullifiers: Vec<_> = p
                    .nullifiers()
                    .iter()
                    .map(|n| hex::encode(n.to_bytes()))
                    .collect();
                state.serialize_field("nullifiers", &nullifiers)?;
            }
            ProtocolTransaction::Moonlight(m) => {
                state.serialize_field("type", "moonlight")?;

                let sender = m.sender();
                let sender = bs58::encode(sender.to_bytes()).into_string();
                state.serialize_field("sender", &sender)?;

                let receiver = m.receiver().map(|receiver| {
                    bs58::encode(receiver.to_bytes()).into_string()
                });
                state.serialize_field("receiver", &receiver)?;

                state.serialize_field("value", &m.value())?;

                state.serialize_field("nonce", &m.nonce())?;
            }
        }

        let tx = &self.inner;

        state.serialize_field("deposit", &tx.deposit())?;

        let notes: Vec<Note> = tx.outputs().iter().map(|n| n.into()).collect();

        if !notes.is_empty() {
            state.serialize_field("outputs", &notes)?;
        }

        let fee = {
            let mut fee = HashMap::new();
            fee.insert("gas_limit", tx.gas_limit().to_string());
            fee.insert("gas_price", tx.gas_price().to_string());

            let encoded_address = match tx.refund_address() {
                RefundAddress::Phoenix(address) => {
                    bs58::encode(address.to_bytes()).into_string()
                }
                RefundAddress::Moonlight(address) => {
                    bs58::encode(address.to_bytes()).into_string()
                }
            };
            fee.insert("refund_address", encoded_address);
            if let ProtocolTransaction::Phoenix(tx) = tx {
                fee.insert(
                    "phoenix sender",
                    hex::encode(tx.sender().to_bytes()),
                );
            }

            fee
        };

        state.serialize_field("fee", &fee)?;

        let call = tx.call().map(|c| {
            let mut call = HashMap::new();
            call.insert("contract", hex::encode(c.contract));
            call.insert("fn_name", c.fn_name.to_string());
            call.insert("fn_args", BASE64_ENGINE.encode(&c.fn_args));
            call
        });
        state.serialize_field("call", &call)?;

        state.serialize_field("is_deploy", &tx.deploy().is_some())?;
        state.serialize_field("memo", &tx.memo().map(hex::encode))?;
        state.end()
    }
}

struct Note<'a>(&'a dusk_core::transfer::phoenix::Note);

impl<'a> From<&'a dusk_core::transfer::phoenix::Note> for Note<'a> {
    fn from(value: &'a dusk_core::transfer::phoenix::Note) -> Self {
        Self(value)
    }
}

impl Serialize for Note<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Note", 5)?;
        let n = self.0;

        state.serialize_field("type", &(n.note_type() as u8))?;

        let commitment = [
            hex::encode(n.value_commitment().get_u().to_bytes()),
            hex::encode(n.value_commitment().get_v().to_bytes()),
        ];
        state.serialize_field("value_commitment", &commitment)?;

        let stealth_address = n.stealth_address().to_bytes();
        state.serialize_field(
            "stealth_address",
            &bs58::encode(stealth_address).into_string(),
        )?;

        state.serialize_field("value_enc", &hex::encode(n.value_enc()))?;
        state.serialize_field("sender", &hex::encode(n.sender().to_bytes()))?;
        state.end()
    }
}
