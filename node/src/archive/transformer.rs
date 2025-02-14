// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::withdraw::WithdrawReceiver;
use dusk_core::transfer::{
    ContractToAccountEvent, ConvertEvent, MoonlightTransactionEvent,
    WithdrawEvent, CONTRACT_TO_ACCOUNT_TOPIC, CONVERT_TOPIC, MINT_TOPIC,
    MOONLIGHT_TOPIC, TRANSFER_CONTRACT, WITHDRAW_TOPIC,
};
use node_data::events::contract::{ContractEvent, ContractTxEvent, OriginHash};
use serde::{Deserialize, Serialize};

/// A group of events that belong to the same transaction.
///
/// This transaction is guaranteed to have changed the balance of at least one
/// public account, therefore seen as a transfer.
#[serde_with::serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct MoonlightTransferEvents {
    events: Vec<ContractEvent>,
}

impl MoonlightTransferEvents {
    // Private on purpose
    fn new(events: Vec<ContractEvent>) -> Self {
        Self { events }
    }

    /// Returns the events of the MoonlightTxEvents.
    ///
    /// This moves the events out of the MoonlightTxEvents.
    pub fn events(self) -> Vec<ContractEvent> {
        self.events
    }
}

/// Transaction hash and block height
#[serde_with::serde_as]
#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct EventIdentifier {
    pub(super) block_height: u64,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(super) tx_hash: OriginHash,
}

impl EventIdentifier {
    pub fn origin(&self) -> &OriginHash {
        &self.tx_hash
    }

    pub fn block_height(&self) -> u64 {
        self.block_height
    }
}

pub(super) type AddressMapping = (AccountPublicKey, EventIdentifier);
pub(super) type MemoMapping = (Vec<u8>, EventIdentifier);
pub(super) struct MoonlightTransferMapping(
    pub EventIdentifier,
    pub MoonlightTransferEvents,
);

pub(super) struct TransormerResult {
    pub address_outflow_mappings: Vec<AddressMapping>,
    pub address_inflow_mappings: Vec<AddressMapping>,
    pub memo_mappings: Vec<MemoMapping>,
    pub moonlight_tx_mappings: Vec<MoonlightTransferMapping>,
}

/// Group a list of events from the same block by origin and block height
pub(super) fn group_by_origins(
    block_events: Vec<ContractTxEvent>,
    block_height: u64,
) -> BTreeMap<EventIdentifier, Vec<ContractEvent>> {
    let mut is_already_grouped: BTreeMap<EventIdentifier, Vec<ContractEvent>> =
        BTreeMap::new();
    for event in block_events {
        let event_to_analyze = event.event;

        is_already_grouped
            .entry(EventIdentifier {
                block_height,
                tx_hash: event.origin,
            })
            .or_default()
            .push(event_to_analyze);
    }
    is_already_grouped
}

/// Returns only the groups that contain a moonlight in- or outflow
///
/// Returns the address mappings, memo mappings and groups
pub(super) fn filter_and_convert(
    grouped_events: BTreeMap<EventIdentifier, Vec<ContractEvent>>,
) -> TransormerResult {
    // TODO: We could add Ord to PublicKey / G2Affine for easy sort & dedup or
    // use inside BTreeMap
    let mut address_inflow_mappings: Vec<(AccountPublicKey, EventIdentifier)> =
        vec![];
    let mut address_outflow_mappings: Vec<(AccountPublicKey, EventIdentifier)> =
        vec![];
    let mut memo_mappings: Vec<(Vec<u8>, EventIdentifier)> = vec![];
    let mut moonlight_tx_mappings = vec![];

    // Iterate over the grouped events and push them to the groups vector in
    // the new format if they are moonlight events
    for (tx_ident, group) in grouped_events {
        let got_recorded = record_flows(
            &mut address_inflow_mappings,
            &mut address_outflow_mappings,
            &mut memo_mappings,
            tx_ident,
            &group,
        );

        if got_recorded {
            moonlight_tx_mappings.push(MoonlightTransferMapping(
                tx_ident,
                MoonlightTransferEvents::new(group),
            ));
        }
    }

    TransormerResult {
        address_outflow_mappings,
        address_inflow_mappings,
        memo_mappings,
        moonlight_tx_mappings,
    }
}

/// Record moonlight inflows/outflows (transfers) from a transaction, based on
/// Event categorization
///
/// # Categories of Events being looked for
///
/// - **Only Moonlight Transaction**
///   - `MOONLIGHT_TOPIC` with `MoonlightTransactionEvent`: Captures all
///     Moonlight protocol transactions (outflows from normal transactions,
///     deposits, or converts). Sender recorded in outflow mapping; receiver in
///     inflow mapping. Refunds (if > 0) are recorded in inflow mapping.
///
/// - **Only Phoenix Transaction**
///   - `CONVERT_TOPIC` with `ConvertEvent`: Receiver recorded in inflow
///     mapping.
///
/// - **Moonlight or Phoenix Transaction**
///   - `WITHDRAW_TOPIC` with `WithdrawEvent`: Receiver recorded in inflow
///     mapping.
///   - `MINT_TOPIC` with `WithdrawEvent`: Receiver recorded in inflow mapping.
///   - `CONTRACT_TO_ACCOUNT_TOPIC` with `ContractToAccountEvent`: Receiver
///     recorded in inflow mapping.
///
///  Mappings are recorded only once per transaction to prevent redundancy.
///
/// # Returns
///
/// Returns true if the group contains a moonlight inflow or outflow, false
/// otherwise.
fn record_flows(
    address_inflow_mappings: &mut Vec<(AccountPublicKey, EventIdentifier)>,
    address_outflow_mappings: &mut Vec<(AccountPublicKey, EventIdentifier)>,
    memo_mappings: &mut Vec<(Vec<u8>, EventIdentifier)>,
    tx_ident: EventIdentifier,
    group: &Vec<ContractEvent>,
) -> bool {
    // Helper to handle inflow mappings without pushing duplicates
    let mut handle_inflow = |key: AccountPublicKey| {
        if !address_inflow_mappings.contains(&(key, tx_ident)) {
            address_inflow_mappings.push((key, tx_ident));
        }
    };

    // Helper to handle outflow mappings without pushing duplicates
    let mut handle_outflow = |key: AccountPublicKey| {
        if !address_outflow_mappings.contains(&(key, tx_ident)) {
            address_outflow_mappings.push((key, tx_ident));
        }
    };

    let filtered_group = group
        .iter()
        .filter(|event| {
            // Make sure that the events originate from the transfer
            // contract.
            if event.target != TRANSFER_CONTRACT {
                return false;
            }

            match event.topic.as_str() {
                MOONLIGHT_TOPIC => {
                    // This also catches deposits or converts.
                    // The DepositEvent or ConvertEvent will have a sender
                    // Some(pk) equal to the sender field of
                    // this MoonlightTransactionEvent
                    let Ok(moonlight_event) = rkyv::from_bytes::<
                        MoonlightTransactionEvent,
                    >(&event.data) else {
                        return false;
                    };

                    // An outflow from the sender pk is always the
                    // case
                    handle_outflow(moonlight_event.sender);

                    // Exhaustively handle all inflow cases
                    // - We don't record refund to sender as inflow (no matter
                    //   which amount)
                    // - We also don't record Zero-refunds to anyone as inflow
                    match (
                        moonlight_event.receiver,
                        moonlight_event.refund_info,
                    ) {
                        (None, refund) => {
                            // If a group only has one event &
                            // the event is "moonlight", it has to be a
                            // transaction to self.
                            if group.len() == 1 {
                                // Tx sent to self are always recorded
                                // as inflows as well (even if value is 0).
                                handle_inflow(moonlight_event.sender);
                            }

                            if let Some((key, amt)) = refund {
                                if amt > 0 {
                                    // We rely on the fact, that refund only
                                    // exists if different from sender
                                    handle_inflow(key);
                                }
                            }
                        }
                        (Some(receiver), None) => {
                            handle_inflow(receiver);
                        }
                        (Some(receiver), Some((key, amt))) => {
                            handle_inflow(receiver);

                            if amt > 0 // We rely on the fact, that refund only exists if different from sender
                                && key != receiver
                            {
                                handle_inflow(key);
                            }
                        }
                    }

                    if !moonlight_event.memo.is_empty() {
                        memo_mappings.push((moonlight_event.memo, tx_ident));
                    }

                    true
                }
                CONVERT_TOPIC => {
                    let Ok(convert_event) =
                        rkyv::from_bytes::<ConvertEvent>(&event.data)
                    else {
                        return false;
                    };

                    let WithdrawReceiver::Moonlight(key) =
                        convert_event.receiver
                    else {
                        return false;
                    };

                    handle_inflow(key);

                    true
                }
                WITHDRAW_TOPIC | MINT_TOPIC => {
                    let Ok(withdraw_event) =
                        rkyv::from_bytes::<WithdrawEvent>(&event.data)
                    else {
                        return false;
                    };

                    let WithdrawReceiver::Moonlight(key) =
                        withdraw_event.receiver
                    else {
                        return false;
                    };

                    handle_inflow(key);

                    true
                }
                CONTRACT_TO_ACCOUNT_TOPIC => {
                    let Ok(contract_to_account_event) =
                        rkyv::from_bytes::<ContractToAccountEvent>(&event.data)
                    else {
                        return false;
                    };

                    handle_inflow(contract_to_account_event.receiver);

                    true
                }
                _ => false,
            }
        })
        .collect::<Vec<&ContractEvent>>();

    !filtered_group.is_empty()
}
