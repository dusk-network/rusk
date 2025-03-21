// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This crate provides a data driver implementation for dusk transfer contract.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

#[cfg(target_family = "wasm")]
mod bindgen;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::transfer::{
    ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
    DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
    WithdrawEvent, CONTRACT_TO_ACCOUNT_TOPIC, CONTRACT_TO_CONTRACT_TOPIC,
    CONVERT_TOPIC, DEPOSIT_TOPIC, MINT_CONTRACT_TOPIC, MINT_TOPIC,
    MOONLIGHT_TOPIC, PHOENIX_TOPIC, WITHDRAW_TOPIC,
};
use dusk_data_driver::{rkyv_to_json, ConvertibleContract, Error, JsonValue};

/// The contract driver for encoding and decoding transactions.
#[derive(Default)]
pub struct ContractDriver;

impl ConvertibleContract for ContractDriver {
    #[allow(unused_variables)]
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        todo!()
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match event_name {
            MOONLIGHT_TOPIC => rkyv_to_json::<MoonlightTransactionEvent>(rkyv),
            PHOENIX_TOPIC => rkyv_to_json::<PhoenixTransactionEvent>(rkyv),
            CONTRACT_TO_CONTRACT_TOPIC | MINT_CONTRACT_TOPIC => {
                rkyv_to_json::<ContractToContractEvent>(rkyv)
            }
            CONTRACT_TO_ACCOUNT_TOPIC => {
                rkyv_to_json::<ContractToAccountEvent>(rkyv)
            }
            WITHDRAW_TOPIC | MINT_TOPIC => rkyv_to_json::<WithdrawEvent>(rkyv),
            DEPOSIT_TOPIC => rkyv_to_json::<DepositEvent>(rkyv),
            CONVERT_TOPIC => rkyv_to_json::<ConvertEvent>(rkyv),

            event => Err(Error::Unsupported(format!("event {event}"))),
        }
    }

    fn get_schema(&self) -> String {
        todo!()
    }
}
