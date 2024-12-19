// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{self, Display};

use dusk_core::transfer::Transaction;
use dusk_core::{dusk, from_dusk};
use rusk_wallet::{BlockTransaction, DecodedNote, GraphQL};

use crate::io::{self};
use crate::settings::Settings;

pub struct TransactionHistory {
    direction: TransactionDirection,
    height: u64,
    amount: f64,
    fee: u64,
    pub tx: Transaction,
    id: String,
}

impl TransactionHistory {
    pub fn header() -> String {
        format!(
            "{: ^9} | {: ^64} | {: ^8} | {: ^17} | {: ^12}",
            "BLOCK", "TX_ID", "METHOD", "AMOUNT", "FEE"
        )
    }
}

impl Display for TransactionHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dusk = self.amount / dusk(1.0) as f64;
        let contract = match self.tx.call() {
            None => "transfer",
            Some(call) => &call.fn_name,
        };

        let fee = match self.direction {
            TransactionDirection::In => "".into(),
            TransactionDirection::Out => {
                let fee = self.fee;
                let fee = from_dusk(fee);
                format!("{: >12.9}", fee)
            }
        };

        let tx_id = &self.id;
        let heigth = self.height;

        write!(
            f,
            "{heigth: >9} | {tx_id} | {contract: ^8} | {dusk: >+17.9} | {fee}",
        )
    }
}

pub(crate) async fn transaction_from_notes(
    settings: &Settings,
    mut notes: Vec<DecodedNote>,
) -> anyhow::Result<Vec<TransactionHistory>> {
    notes.sort_by(|a, b| a.note.pos().cmp(b.note.pos()));
    let mut ret: Vec<TransactionHistory> = vec![];
    let gql =
        GraphQL::new(settings.state.to_string(), io::status::interactive)?;

    let nullifiers = notes
        .iter()
        .flat_map(|note| {
            note.nullified_by.map(|nullifier| (nullifier, note.amount))
        })
        .collect::<Vec<_>>();

    let mut block_txs = HashMap::new();

    for mut decoded_note in notes {
        // Set the position to max, in order to match the note with the one
        // in the tx
        decoded_note.note.set_pos(u64::MAX);

        let note_amount = decoded_note.amount as f64;

        let txs = match block_txs.entry(decoded_note.block_height) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let txs = gql.txs_for_block(decoded_note.block_height).await?;
                v.insert(txs)
            }
        };

        let note_hash = decoded_note.note.hash();
        // Looking for the transaction which created the note
        let note_creator = txs.iter_mut().find(|tx_block| {
            let tx = &tx_block.tx;

            tx.outputs().iter().any(|note| note.hash().eq(&note_hash))
                || tx.nullifiers().iter().any(|tx_null| {
                    nullifiers.iter().any(|(nullifier, _)| nullifier == tx_null)
                })
        });
        if let Some(BlockTransaction { tx, id, gas_spent }) = note_creator {
            let inputs_amount: f64 = tx
                .nullifiers()
                .iter()
                .filter_map(|input| {
                    nullifiers.iter().find_map(|(nullifier, gas)| {
                        nullifier.eq(input).then_some(gas)
                    })
                })
                .sum::<u64>() as f64;

            let direction = match inputs_amount > 0f64 {
                true => TransactionDirection::Out,
                false => TransactionDirection::In,
            };

            match ret.iter_mut().find(|th| &th.id == id) {
                Some(tx) => tx.amount += note_amount,
                None => ret.push(TransactionHistory {
                    direction,
                    height: decoded_note.block_height,
                    amount: note_amount - inputs_amount,
                    fee: *gas_spent * tx.gas_price(),
                    tx: tx.clone(),
                    id: id.clone(),
                }),
            }
        } else {
            let outgoing_tx = ret.iter_mut().find(|th| {
                th.direction == TransactionDirection::Out
                    && th.height == decoded_note.block_height
            });

            match outgoing_tx {
                // Outgoing txs found, this should be the change or any
                // other output created by the tx result
                // (like withdraw or unstake)
                Some(th) => th.amount += note_amount,

                // No outgoing txs found, this note should belong to a
                // preconfigured genesis state
                None => println!("??? val {}", note_amount),
            }
        }
    }
    ret.sort_by(|a, b| a.height.cmp(&b.height));
    Ok(ret)
}

#[derive(PartialEq)]
enum TransactionDirection {
    In,
    Out,
}
