// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::ledger::{Hash, SpentTransaction, Transaction};

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
            Self::Executed(t) => Some(t.to_json()),
            Self::Included(t) => Some(t.to_json()),
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
use serde_json::{json, Map};

impl Transaction {
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = Map::new();
        let t = &self.inner;
        match t {
            ProtocolTransaction::Phoenix(_) => {
                map.insert("_type".into(), json!("phoenix"));
            }
            ProtocolTransaction::Moonlight(_) => {
                map.insert("_type".into(), json!("moonlight"));
            }
        }

        let tx = &self.inner;
        if let Some(root) = tx.root() {
            map.insert("root".into(), json!(hex::encode(root.to_bytes())));
        }

        if let Some(from) = tx.from() {
            map.insert("from".into(), json!(hex::encode(from.to_bytes())));
        }
        if let Some(to) = tx.to() {
            map.insert("to".into(), json!(hex::encode(to.to_bytes())));
        }
        if let Some(value) = tx.value() {
            map.insert("value".into(), json!(hex::encode(value.to_bytes())));
        }

        let nullifiers: Vec<_> = tx
            .nullifiers()
            .iter()
            .map(|n| hex::encode(n.to_bytes()))
            .collect();
        if !nullifiers.is_empty() {
            map.insert("nullifiers".into(), json!(nullifiers));
        }
        map.insert(
            "deposit".into(),
            json!(hex::encode(tx.deposit().to_bytes())),
        );
        let notes: Vec<_> = tx
            .outputs()
            .iter()
            .map(|n| {
                let mut map = Map::new();
                map.insert("note_type".into(), json!(n.note_type() as u8));
                map.insert(
                    "value_commitment".into(),
                    json!([
                        hex::encode(n.value_commitment().get_u().to_bytes()),
                        hex::encode(n.value_commitment().get_v().to_bytes())
                    ]),
                );
                map.insert(
                    "stealth_address".into(),
                    json!(bs58::encode(n.stealth_address().to_bytes())
                        .into_string()),
                );
                map.insert(
                    "value_enc".into(),
                    json!(n
                        .value_enc()
                        .iter()
                        .map(|c| hex::encode(c.to_bytes()))
                        .collect::<Vec<_>>()),
                );
                map.insert(
                    "sender".into(),
                    json!(hex::encode(n.sender().to_bytes())),
                );
                map
            })
            .collect();
        if !notes.is_empty() {
            map.insert("notes".into(), json!(notes));
        }

        let fee = {
            let mut fee = Map::new();
            fee.insert("gas_limit".into(), json!(tx.gas_limit()));
            fee.insert("gas_price".into(), json!(tx.gas_price()));

            if let Some(stealth_address) = tx.stealth_address() {
                fee.insert(
                    "stealth_address".into(),
                    json!(
                        bs58::encode(stealth_address.to_bytes()).into_string()
                    ),
                );
            }
            if let Some(sender) = tx.sender() {
                fee.insert(
                    "sender".into(),
                    json!(hex::encode(sender.to_bytes())),
                );
            }
            fee
        };

        map.insert("fee".into(), json!(fee));

        if let Some(c) = tx.call() {
            let mut call = Map::new();
            call.insert("contract".into(), json!(hex::encode(c.contract)));
            call.insert("fn_name".into(), json!(&c.fn_name));
            call.insert("fn_args".into(), json!(hex::encode(&c.fn_args)));
            map.insert("call".into(), json!(call));
        }
        json!(map)
    }
}

impl SpentTransaction {
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = Map::new();
        map.insert("blockHeight".into(), json!(self.block_height));
        map.insert("gasSpent".into(), json!(self.gas_spent));
        map.insert("err".into(), json!(self.err));
        map.insert("tx".into(), self.inner.to_json());
        json!(map)
    }
}
