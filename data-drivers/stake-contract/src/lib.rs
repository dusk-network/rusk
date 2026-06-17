// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This crate provides a data driver implementation for dusk stake contract.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    Reward, SlashEvent, Stake, StakeConfig, StakeData, StakeEvent, StakeKeys,
    Withdraw, WithdrawToContract,
};
use dusk_core::transfer::ReceiveFromContract;
use dusk_data_driver::{
    json_to_rkyv, rkyv_to_json, rkyv_to_json_u64, ConvertibleContract, Error,
    JsonValue,
};

/// The contract driver for encoding and decoding transactions.
#[derive(Default)]
pub struct ContractDriver;

const STAKE_SCHEMA: &str = r#"{"name":"StakeContract","imports":[],"functions":[{"name":"stake","doc":"","input":"Stake","output":"()","custom":false},{"name":"unstake","doc":"","input":"Withdraw","output":"()","custom":false},{"name":"withdraw","doc":"","input":"Withdraw","output":"()","custom":false},{"name":"stake_from_contract","doc":"","input":"ReceiveFromContract","output":"()","custom":false},{"name":"unstake_from_contract","doc":"","input":"WithdrawToContract","output":"()","custom":false},{"name":"withdraw_from_contract","doc":"","input":"WithdrawToContract","output":"()","custom":false},{"name":"get_stake","doc":"","input":"BlsPublicKey","output":"Option<StakeData>","custom":false},{"name":"get_stake_keys","doc":"","input":"BlsPublicKey","output":"Option<StakeKeys>","custom":false},{"name":"burnt_amount","doc":"","input":"()","output":"u64","custom":false},{"name":"get_version","doc":"","input":"()","output":"u64","custom":false},{"name":"get_config","doc":"","input":"()","output":"StakeConfig","custom":false},{"name":"stakes","doc":"","input":"()","output":"(StakeKeys,StakeData)","custom":false},{"name":"prev_state_changes","doc":"","input":"()","output":"(BlsPublicKey,Option<StakeData>)","custom":false}],"events":[{"topic":"stake","data":"StakeEvent"},{"topic":"unstake","data":"StakeEvent"},{"topic":"withdraw","data":"StakeEvent"},{"topic":"reward","data":"Vec<Reward>"},{"topic":"slash","data":"SlashEvent"},{"topic":"hard_slash","data":"SlashEvent"}]}"#;

impl ConvertibleContract for ContractDriver {
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        match fn_name {
            // Transactions
            "stake" => json_to_rkyv::<Stake>(json),
            "unstake" | "withdraw" => json_to_rkyv::<Withdraw>(json),
            "stake_from_contract" => json_to_rkyv::<ReceiveFromContract>(json),
            "unstake_from_contract" | "withdraw_from_contract" => {
                json_to_rkyv::<WithdrawToContract>(json)
            }
            // Queries (TBD)
            "get_stake" | "get_stake_keys" => {
                json_to_rkyv::<BlsPublicKey>(json)
            }
            "burnt_amount" | "get_version" | "get_config" | "stakes"
            | "prev_state_changes" => json_to_rkyv::<()>("null"),

            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match fn_name {
            // Transactions
            "stake" => rkyv_to_json::<Stake>(rkyv),
            "unstake" | "withdraw" => rkyv_to_json::<Withdraw>(rkyv),
            "stake_from_contract" => rkyv_to_json::<ReceiveFromContract>(rkyv),
            "unstake_from_contract" | "withdraw_from_contract" => {
                rkyv_to_json::<WithdrawToContract>(rkyv)
            }
            // Queries (TBD)
            "get_stake" | "get_stake_keys" => {
                rkyv_to_json::<BlsPublicKey>(rkyv)
            }
            "burnt_amount" | "get_version" | "get_config" | "stakes"
            | "prev_state_changes" => rkyv_to_json::<()>(rkyv),

            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match fn_name {
            // Transactions
            "stake"
            | "unstake"
            | "withdraw"
            | "stake_from_contract"
            | "unstake_from_contract"
            | "withdraw_from_contract" => Ok(JsonValue::Null),
            // Queries (TBD)
            "get_stake" => rkyv_to_json::<Option<StakeData>>(rkyv),
            "get_stake_keys" => rkyv_to_json::<Option<StakeKeys>>(rkyv),
            "burnt_amount" | "get_version" => rkyv_to_json_u64(rkyv),
            "get_config" => rkyv_to_json::<StakeConfig>(rkyv),

            "stakes" => rkyv_to_json::<(StakeKeys, StakeData)>(rkyv),
            "prev_state_changes" => {
                rkyv_to_json::<(BlsPublicKey, Option<StakeData>)>(rkyv)
            }

            // Unsupported
            name => Err(Error::Unsupported(format!("fn_name {name}"))),
        }
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        match event_name {
            "stake" | "unstake" | "withdraw" => {
                rkyv_to_json::<StakeEvent>(rkyv)
            }
            "reward" => rkyv_to_json::<Vec<Reward>>(rkyv),
            "slash" | "hard_slash" => rkyv_to_json::<SlashEvent>(rkyv),
            event => Err(Error::Unsupported(format!("event {event}"))),
        }
    }

    fn get_schema(&self) -> String {
        STAKE_SCHEMA.into()
    }
}

#[cfg(all(target_family = "wasm", feature = "ffi"))]
dusk_data_driver::generate_wasm_entrypoint!(ContractDriver);
