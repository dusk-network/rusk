// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;

use super::*;
use crate::ledger::{Hash, SpentTransaction, Transaction};

/// Represents events related to transactions.
///
/// - `Removed(Hash)`
///
///     Indicates that a transaction has been removed from the mempool. The
///     `Hash` represents the unique identifier of the removed transaction.
///
/// - `Included(&'t Transaction)`
///
///     A transaction has been included in the mempool.
///
/// - `Executed(&'t SpentTransaction)`
///
///     Denotes that a transaction has been executed into an accepted block.
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
            Self::Executed(tx) => tx.inner.hash(),
            Self::Included(tx) => tx.hash(),
        };
        hex::encode(hash)
    }
}
use dusk_bytes::Serializable;
use execution_core::transfer::Transaction as ProtocolTransaction;

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for Transaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Transaction", 1)?;
        let t = &self.inner;
        match t {
            ProtocolTransaction::Phoenix(_) => {
                state.serialize_field("type", "phoenix")?;

                let root = t.root().expect("phoenix to have root");
                state.serialize_field("root", &hex::encode(root.to_bytes()))?;

                let nullifiers: Vec<_> = t
                    .nullifiers()
                    .iter()
                    .map(|n| hex::encode(n.to_bytes()))
                    .collect();
                if !nullifiers.is_empty() {
                    state.serialize_field("nullifiers", &nullifiers)?;
                }
            }
            ProtocolTransaction::Moonlight(_) => {
                state.serialize_field("type", "moonlight")?;

                let from = t.from().expect("moonlight to have from");
                let from = bs58::encode(from.to_bytes()).into_string();
                state.serialize_field("from", &from)?;

                let to = t.to().expect("moonlight to have to");
                let to = bs58::encode(to.to_bytes()).into_string();
                state.serialize_field("to", &to)?;

                let value = t.value().expect("moonlight to have value");
                state.serialize_field("value", &value)?;
            }
        }

        let tx = &self.inner;

        state.serialize_field("deposit", &tx.deposit())?;

        let notes: Vec<Note> = tx.outputs().iter().map(|n| n.into()).collect();

        if !notes.is_empty() {
            state.serialize_field("notes", &notes)?;
        }

        let fee = {
            let mut fee = HashMap::new();
            fee.insert("gas_limit", tx.gas_limit().to_string());
            fee.insert("gas_price", tx.gas_price().to_string());

            if let Some(stealth_address) = tx.stealth_address() {
                fee.insert(
                    "stealth_address",
                    bs58::encode(stealth_address.to_bytes()).into_string(),
                );
            }
            if let Some(sender) = tx.sender() {
                fee.insert("sender", hex::encode(sender.to_bytes()));
            }
            fee
        };

        state.serialize_field("fee", &fee)?;

        let call = tx.call().map(|c| {
            let mut call = HashMap::new();
            call.insert("contract", hex::encode(c.contract));
            call.insert("fn_name", c.fn_name.to_string());
            call.insert("fn_args", base64::encode(&c.fn_args));
            call
        });
        state.serialize_field("call", &call)?;
        state.end()
    }
}

struct Note<'a>(&'a execution_core::transfer::phoenix::Note);

impl<'a> From<&'a execution_core::transfer::phoenix::Note> for Note<'a> {
    fn from(value: &'a execution_core::transfer::phoenix::Note) -> Self {
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
