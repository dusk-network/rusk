// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{self, Display};

use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::withdraw::WithdrawReceiver;
use dusk_core::transfer::{Transaction, TRANSFER_CONTRACT};
use dusk_core::{dusk, from_dusk};
use rusk_wallet::{Address, BlockData, BlockTransaction, DecodedNote, GraphQL};

use crate::io::{self};
use crate::settings::Settings;

#[derive(Debug, PartialEq)]
pub struct TransactionHistory {
    pub(crate) direction: TransactionDirection,
    pub(crate) height: u64,
    pub(crate) amount: f64,
    pub(crate) fee: u64,
    pub(crate) tx: Transaction,
    pub(crate) id: String,
    pub(crate) bal_type: BalanceType,
}

impl TransactionHistory {
    pub fn header() -> String {
        format!(
            "{: ^9} | {: ^64} | {: ^8} | {: ^17} | {: ^12} | {: ^8}\n",
            "BLOCK", "TX_ID", "ACTION", "AMOUNT", "FEE", "BALANCE_TYPE"
        )
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub(crate) fn action(&self) -> &str {
        match self.tx.call() {
            None => "transfer",
            Some(call) => {
                if call.contract == STAKE_CONTRACT && call.fn_name == "withdraw"
                {
                    "claim-reward"
                } else {
                    &call.fn_name
                }
            }
        }
    }
}

impl Display for TransactionHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dusk = self.amount / dusk(1.0) as f64;
        let action = self.action();

        let fee = match self.direction {
            TransactionDirection::In => format!("{: >12.9}", ""),
            TransactionDirection::Out => {
                let fee: u64 = self.fee;
                let fee = from_dusk(fee);
                format!("{: >12.9}", -fee)
            }
        };

        let tx_id = &self.id;
        let height = self.height;
        let bal_type = &self.bal_type;

        writeln!(
            f,
            "{height: >9} | {tx_id} | {action: ^8} | {dusk: >+17.9} | {fee} | {bal_type}",
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
    let gql = GraphQL::new(
        settings.state.to_string(),
        settings.archiver.to_string(),
        io::status::interactive,
    )?;

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
                        bal_type: BalanceType::Shielded,
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
                // (like claim rewards or unstake)
                th.amount += note_amount;
            } else {
                // No outgoing txs found, this note should either belong to a
                // preconfigured genesis state or is the result of a
                // moonlight to phoenix conversion.
                if decoded_note.block_height == 0 {
                    continue;
                }
                let moonlight_tx_events = gql
                    .moonlight_history_at_block(
                        public_address,
                        decoded_note.block_height,
                    )
                    .await?;
                let moonlight_history = moonlight_tx_events
                    .full_moonlight_history
                    .ok_or(anyhow::anyhow!("Couldn't find the transaction that created a note for the history."))?;
                let note_creator = moonlight_history.json
                    .iter()
                    .find_map(|history_info| history_info.events.iter().find_map(|event| {
                        if let BlockData::ConvertEvent(ref event) = event.data {
                            if let WithdrawReceiver::Phoenix(receiver_address) = event.receiver {
                                if decoded_note.note.stealth_address() == &receiver_address {
                                    // The note is the output of a moonlight
                                    // to phoenix conversion.
                                    let tx = txs.iter().find(|block_tx| {
                                        block_tx.id == history_info.origin
                                    }).expect("The transaction should be in this list since it's the list of all transactions at its block height.");
                                    return Some(tx);
                                }
                            }
                        }
                        None
                    }))
                    .ok_or(anyhow::anyhow!("Couldn't find the transaction that created a note for the history."))?;
                ret.push(TransactionHistory {
                    direction: TransactionDirection::In,
                    height: decoded_note.block_height,
                    amount: decoded_note.amount as f64,
                    fee: 0,
                    tx: note_creator.tx.clone(),
                    id: note_creator.id.clone(),
                    bal_type: BalanceType::Shielded,
                });
            }
        }
    }

    Ok(ret)
}

pub(crate) async fn moonlight_history(
    settings: &Settings,
    address: rusk_wallet::Address,
) -> anyhow::Result<Vec<TransactionHistory>> {
    let gql = GraphQL::new(
        settings.state.to_string(),
        settings.archiver.to_string(),
        io::status::interactive,
    )?;

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

        // Any event could be emitted by a third party contract.
        // Only events emitted by the transfer contract are trustworthy.
        for event in events
            .into_iter()
            .filter(|event| event.target == TRANSFER_CONTRACT)
        {
            match event.data {
                BlockData::MoonlightTransactionEvent(event) => {
                    // This event comes up as the sole event the the case of
                    // moonlight transfers. In other kinds of transactions, this
                    // event comes up along with one or more
                    // events.
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
                    if direction.is_none() {
                        // It could have been set while handling some other
                        // event.
                        direction = Some(event_direction);
                    }
                }
                BlockData::ConvertEvent(event) => {
                    // This comes up in phoenix to moonlight conversions
                    // and moonlight to phoenix conversions.
                    //
                    // In phoenix to moonlight conversions, both this
                    // `ConvertEvent`
                    // and `PhoenixTransactionEvent` are emitted.
                    // In moonlight to phoenix conversions, both this
                    // `ConvertEvent`
                    // and `MoonlightTransactionEvent` are emitted.
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
                    // In the full moonlight history, this
                    // comes up in a phoenix to moonlight conversion.
                    // In this conversion, a `PhoenixTransactionEvent`
                    // and `ConvertEvent` are emitted.
                    //
                    // This case has already been handled in
                    // `transaction_from_notes`
                }
                BlockData::DepositEvent(event) => {
                    // This event is emitted when funds are deposited
                    // in a contract.
                    //
                    // For public stake events: the value
                    // staked is deposited in the stake contract.
                    // This event contains the amount deposited. All
                    // other info required for the history is in the
                    // `MoonlightTransactionEvent`.
                    amount -= event.value as f64;
                    direction = Some(TransactionDirection::Out);
                }
                BlockData::StakeEvent(_) => {
                    // When a public stake is done, three events are emitted:
                    // a `MoonlightTransactionEvent`, a `StakeEvent` and a
                    // `DepositEvent`.
                    // When a public unstake is done, three events are emitted:
                    // a `MoonlightTransactionEvent`, a `StakeEvent` and a
                    // `WithdrawEvent`.
                    //
                    // In both cases, the moonlight transaction event
                    // and deposit/withdraw event handle everything so
                    // there is no need to do anything here.
                    unreachable!("Because all non-transfer contract events are filtered out.")
                }
                BlockData::WithdrawEvent(event) => {
                    // This event is emitted when funds are withdrawn
                    // from a contract.
                    //
                    // For public unstake events: the value unstaked is
                    // withdrawn from the stake contract. This event contains
                    // only the amount withdrawn. All other info required for
                    // the history is in the `MoonlightTransactionEvent`.
                    amount += event.value as f64;
                    direction = Some(TransactionDirection::Out);
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
            bal_type: BalanceType::Public,
        });
    }

    Ok(collected_history)
}

#[derive(PartialEq, Debug)]
pub(crate) enum TransactionDirection {
    In,
    Out,
}

#[derive(PartialEq, Debug)]
pub(crate) enum BalanceType {
    Shielded,
    Public,
}

impl Display for BalanceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shielded => write!(f, "shielded"),
            Self::Public => write!(f, "public"),
        }
    }
}
