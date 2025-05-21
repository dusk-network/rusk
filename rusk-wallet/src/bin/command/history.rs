// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{self, Display};

use dusk_core::transfer::withdraw::WithdrawReceiver;
use dusk_core::transfer::Transaction;
use dusk_core::{dusk, from_dusk};
use rusk_wallet::{Address, BlockData, BlockTransaction, DecodedNote, GraphQL};

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
            "{: ^9} | {: ^64} | {: ^8} | {: ^17} | {: ^12} | {: ^8}\n",
            "BLOCK", "TX_ID", "METHOD", "AMOUNT", "FEE", "TRANSACTION_TYPE"
        )
    }

    pub fn height(&self) -> u64 {
        self.height
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
            TransactionDirection::In => format!("{: >12.9}", ""),
            TransactionDirection::Out => {
                let fee: u64 = self.fee;
                let fee = from_dusk(fee);
                format!("{: >12.9}", fee)
            }
        };

        let tx_id = &self.id;
        let height = self.height;

        let tx_type = match self.tx {
            Transaction::Moonlight(_) => dusk_core::transfer::MOONLIGHT_TOPIC,
            Transaction::Phoenix(_) => dusk_core::transfer::PHOENIX_TOPIC,
        };

        writeln!(
            f,
            "{height: >9} | {tx_id} | {contract: ^8} | {dusk: >+17.9} | {fee} | {tx_type}",
        )
    }
}

pub(crate) async fn transaction_from_notes(
    settings: &Settings,
    mut notes: Vec<DecodedNote>,
    public_address: &Address,
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
                None => {
                    let fee = *gas_spent * tx.gas_price();
                    let amount = note_amount - inputs_amount;
                    let amount = if direction == TransactionDirection::Out {
                        // The fee has already been subtracted from the amount
                        // and it should not be included in the amount.
                        amount + fee as f64
                    } else {
                        amount
                    };
                    ret.push(TransactionHistory {
                        direction,
                        height: decoded_note.block_height,
                        amount,
                        fee,
                        tx: tx.clone(),
                        id: id.clone(),
                    })
                }
            }
        } else {
            let outgoing_tx = ret.iter_mut().find(|th| {
                th.direction == TransactionDirection::Out
                    && th.height == decoded_note.block_height
            });

            if let Some(th) = outgoing_tx {
                // Outgoing txs found, this should be the change or any
                // other output created by the tx result
                // (like withdraw or unstake)
                th.amount += note_amount;
            } else {
                // No outgoing txs found, this note should either belong to a
                // preconfigured genesis state or is the result of a
                // moonlight to phoenix conversion.
                let moonlight_tx_events = gql
                    .moonlight_history_at_block(
                        public_address,
                        decoded_note.block_height,
                    )
                    .await?;
                let Some(moonlight_history) =
                    moonlight_tx_events.full_moonlight_history
                else {
                    continue;
                };
                for history_info in moonlight_history.json {
                    for event in history_info.events {
                        if let BlockData::ConvertEvent(event) = event.data {
                            if let WithdrawReceiver::Phoenix(receiver_address) =
                                event.receiver
                            {
                                if decoded_note.note.stealth_address()
                                    == &receiver_address
                                {
                                    // The note is the output of a moonlight to
                                    // phoenix conversion.
                                    let note_creator = txs.iter().find(|block_tx| {
                                        block_tx.id == history_info.origin
                                    }).expect("The transaction should be in this list since it's the list of all transactions at its block height.");
                                    ret.push(TransactionHistory {
                                        direction: TransactionDirection::In,
                                        height: decoded_note.block_height,
                                        amount: event.value as f64,
                                        fee: 0,
                                        tx: note_creator.tx.clone(),
                                        id: history_info.origin.clone(),
                                    });
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ret)
}

pub(crate) async fn moonlight_history(
    settings: &Settings,
    address: rusk_wallet::Address,
) -> anyhow::Result<Vec<TransactionHistory>> {
    let gql =
        GraphQL::new(settings.state.to_string(), io::status::interactive)?;

    let history = gql
        .moonlight_history(address.clone())
        .await?
        .full_moonlight_history;
    let Some(history) = history else {
        return Ok(vec![]);
    };

    let mut collected_history = Vec::new();

    for history_item in history.json {
        let id = history_item.origin;
        let events = history_item.events;
        let height = history_item.block_height;
        let tx = gql.moonlight_tx(&id).await?;
        let mut amount = 0.0;
        let mut direction = None;
        let mut fee = 0;

        for event in events {
            match event.data {
                BlockData::MoonlightTransactionEvent(event) => {
                    fee = event.gas_spent * tx.gas_price();
                    let sender = event.sender;
                    let (event_direction, event_amount) = match &sender
                        == address.public_key()?
                    {
                        true => {
                            (TransactionDirection::Out, -(event.value as f64))
                        }
                        false => (TransactionDirection::In, event.value as f64),
                    };
                    amount += event_amount;
                    direction = Some(event_direction);
                }
                BlockData::ConvertEvent(event) => {
                    let (event_direction, event_amount) = match event
                        .sender
                        .is_some()
                        && &event.sender.unwrap() == address.public_key()?
                    {
                        true => {
                            (TransactionDirection::Out, -(event.value as f64))
                        }
                        false => (TransactionDirection::In, event.value as f64),
                    };
                    direction = Some(event_direction);
                    amount += event_amount;
                }
                BlockData::PhoenixTransactionEvent(_) => {
                    // This case has already been handled in
                    // `transaction_from_notes`
                }
            }
        }
        collected_history.push(TransactionHistory {
            direction: direction.expect("should be determined"),
            height,
            amount,
            tx: tx.clone(),
            id: id.clone(),
            fee,
        });
    }

    Ok(collected_history)
}

#[derive(PartialEq, Debug)]
enum TransactionDirection {
    In,
    Out,
}
