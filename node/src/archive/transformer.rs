// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use execution_core::transfer::withdraw::WithdrawReceiver;
use execution_core::transfer::{
    ConvertEvent, MoonlightTransactionEvent, WithdrawEvent, CONVERT_TOPIC,
    MINT_TOPIC, MOONLIGHT_TOPIC, TRANSFER_CONTRACT, WITHDRAW_TOPIC,
};
use node_data::events::contract::{ContractEvent, ContractTxEvent, TxHash};
use serde::{Deserialize, Serialize};

/// More efficient format for events that belong to the same tx to not duplicate
/// TxHash
#[serde_with::serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonlightTxEvents {
    events: Vec<ContractEvent>,
    #[serde_as(as = "serde_with::hex::Hex")]
    origin: TxHash,
}

impl MoonlightTxEvents {
    // Private on purpose
    fn new(events: Vec<ContractEvent>, origin: TxHash) -> Self {
        Self { events, origin }
    }

    pub fn events(&self) -> &Vec<ContractEvent> {
        &self.events
    }

    pub fn origin(&self) -> &TxHash {
        &self.origin
    }
}

type AddressMapping = (AccountPublicKey, TxHash);
type MemoMapping = (Vec<u8>, TxHash);

/// Groups the events from a block by their origin and returns
/// only the groups that contain a moonlight in- or outflow
///
/// Returns the address mappings, memo mappings and groups
pub(super) fn group_by_origins_filter_and_convert(
    block_events: Vec<ContractTxEvent>,
) -> (
    Vec<AddressMapping>,
    Vec<MemoMapping>,
    Vec<MoonlightTxEvents>,
) {
    // 1st Group events by origin (TxHash) & throw away the ones that
    // don't have an origin
    let mut moonlight_is_already_grouped: BTreeMap<TxHash, Vec<ContractEvent>> =
        BTreeMap::new();
    for event in block_events {
        if let Some(origin) = event.origin {
            let event_to_analyze = event.event;

            moonlight_is_already_grouped
                .entry(origin)
                .or_default()
                .push(event_to_analyze);
        }
    }

    // 2nd Keep only the event groups which contain a moonlight in-
    // or outflow

    let mut address_mappings: Vec<(AccountPublicKey, TxHash)> = vec![];
    let mut memo_mappings: Vec<(Vec<u8>, TxHash)> = vec![];
    let mut moonlight_tx_groups = vec![];
    // Iterate over the grouped events and push them to the groups vector in
    // the new format if they are moonlight events
    for (tx_hash, group) in moonlight_is_already_grouped {
        let is_moonlight = group.iter().any(|event| {
            // Make sure that the events originate from the transfer contract.
            if event.target.0 != TRANSFER_CONTRACT {
                return false;
            }

            /*
            Cases of a moonlight in- or outflow:
            1. Any MoonlightTransactionEvent. This implicitly also catches a moonlight outflow for deposit or convert (from moonlight)
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
                        address_mappings
                            .push((moonlight_event.sender, tx_hash));
                        if let Some(receiver) = moonlight_event.receiver {
                            if moonlight_event.sender != receiver {
                                // don't push if tx is sent to self
                                address_mappings.push((receiver, tx_hash));
                            }
                        }

                        if !moonlight_event.memo.is_empty() {
                            memo_mappings.push((moonlight_event.memo, tx_hash));
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
                            address_mappings.push((key, tx_hash));
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
                            address_mappings.push((key, tx_hash));
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            }
        });

        if is_moonlight {
            moonlight_tx_groups.push(MoonlightTxEvents::new(group, tx_hash));
        }
    }

    (address_mappings, memo_mappings, moonlight_tx_groups)
}
