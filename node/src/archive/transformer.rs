// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::withdraw::WithdrawReceiver;
use dusk_core::transfer::{
    ConvertEvent, MoonlightTransactionEvent, WithdrawEvent, CONVERT_TOPIC,
    MINT_TOPIC, MOONLIGHT_TOPIC, TRANSFER_CONTRACT, WITHDRAW_TOPIC,
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
    // TODO: We could add Ord to PublicKey / G2Affine for easy sort & dedup or use
    // inside BTreeMap
    let mut address_inflow_mappings: Vec<(AccountPublicKey, EventIdentifier)> =
        vec![];
    let mut address_outflow_mappings: Vec<(AccountPublicKey, EventIdentifier)> =
        vec![];
    let mut memo_mappings: Vec<(Vec<u8>, EventIdentifier)> = vec![];
    let mut moonlight_tx_mappings = vec![];
    // Iterate over the grouped events and push them to the groups vector in
    // the new format if they are moonlight events
    for (tx_ident, group) in grouped_events {
        let is_moonlight = group
            .iter()
            .filter(|event| {
                // Make sure that the events originate from the transfer
                // contract.
                if event.target.0 != TRANSFER_CONTRACT {
                    return false;
                }

                /*
                Cases of a moonlight in- or outflow:
                1. Any MoonlightTransactionEvent. This implicitly also catches a moonlight outflow for deposit, convert or refund (from moonlight)
                2a. Any withdraw event where the receiver is moonlight. (from phoenix)
                2b. Any mint event where the receiver is moonlight. (from staking)
                3. Any convert event where the receiver is moonlight. (from phoenix)
                */
                match event.topic.as_str() {
                    MOONLIGHT_TOPIC => {
                        /*
                            This also catches deposits & converts.
                            For deposits & convert the sender will be Some(pk) where pk is the same as the from field of the MoonlightTransactionEvent
                        */
                        if let Ok(moonlight_event) =
                            rkyv::from_bytes::<MoonlightTransactionEvent>(
                                &event.data,
                            )
                        {
                            // An outflow from the sender address is always the
                            // case
                            if !address_outflow_mappings
                                .contains(&(moonlight_event.sender, tx_ident))
                            {
                                address_outflow_mappings
                                    .push((moonlight_event.sender, tx_ident));
                            }

                            // Exhaustively handle all inflow cases
                            match (
                                moonlight_event.receiver,
                                moonlight_event.refund_info,
                            ) {
                                (None, refund) => {
                                    // Note: Tx sent to self are also recorded
                                    // as
                                    // inflows.
                                    // If a group only has one event & the event
                                    // is
                                    // "moonlight", it has to be a transaction
                                    // to
                                    // self.
                                    if group.len() == 1
                                        && !address_inflow_mappings.contains(&(
                                            moonlight_event.sender,
                                            tx_ident,
                                        ))
                                    {
                                        address_inflow_mappings.push((
                                            moonlight_event.sender,
                                            tx_ident,
                                        ));
                                    }

                                    // addr != moonlight_event.sender to not
                                    // record
                                    // an inflow twice for the same tx
                                    if let Some((addr, amt)) = refund {
                                        if amt > 0
                                            && addr != moonlight_event.sender
                                            && !address_inflow_mappings
                                                .contains(&(addr, tx_ident))
                                        {
                                            address_inflow_mappings
                                                .push((addr, tx_ident));
                                        }
                                    }
                                }
                                (Some(receiver), None) => {
                                    if !address_inflow_mappings
                                        .contains(&(receiver, tx_ident))
                                    {
                                        address_inflow_mappings
                                            .push((receiver, tx_ident))
                                    }
                                }
                                (Some(receiver), Some((addr, amt))) => {
                                    if !address_inflow_mappings
                                        .contains(&(receiver, tx_ident))
                                    {
                                        address_inflow_mappings
                                            .push((receiver, tx_ident));
                                    }

                                    if amt > 0
                                        && addr != receiver
                                        && addr != moonlight_event.sender
                                        && !address_inflow_mappings
                                            .contains(&(addr, tx_ident))
                                    {
                                        address_inflow_mappings
                                            .push((addr, tx_ident));
                                    }
                                }
                            }

                            if !moonlight_event.memo.is_empty() {
                                memo_mappings
                                    .push((moonlight_event.memo, tx_ident));
                            }

                            return true;
                        }
                        false
                    }
                    WITHDRAW_TOPIC | MINT_TOPIC => {
                        if let Ok(withdraw_event) =
                            rkyv::from_bytes::<WithdrawEvent>(&event.data)
                        {
                            if let WithdrawReceiver::Moonlight(key) =
                                withdraw_event.receiver
                            {
                                if !address_inflow_mappings
                                    .contains(&(key, tx_ident))
                                {
                                    address_inflow_mappings
                                        .push((key, tx_ident));
                                }
                                return true;
                            }
                        }
                        false
                    }
                    CONVERT_TOPIC => {
                        if let Ok(convert_event) =
                            rkyv::from_bytes::<ConvertEvent>(&event.data)
                        {
                            if let WithdrawReceiver::Moonlight(key) =
                                convert_event.receiver
                            {
                                if !address_inflow_mappings
                                    .contains(&(key, tx_ident))
                                {
                                    address_inflow_mappings
                                        .push((key, tx_ident));
                                }
                                return true;
                            }
                        }
                        false
                    }
                    _ => false,
                }
            })
            .collect::<Vec<&ContractEvent>>();

        if !is_moonlight.is_empty() {
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
