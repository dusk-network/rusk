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

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::NoteLeaf;
use dusk_core::transfer::{
    withdraw::Withdraw, ContractToAccount, ContractToAccountEvent,
    ContractToContract, ContractToContractEvent, ConvertEvent, DepositEvent,
    MoonlightTransactionEvent, PhoenixTransactionEvent, WithdrawEvent,
    CONTRACT_TO_ACCOUNT_TOPIC, CONTRACT_TO_CONTRACT_TOPIC, CONVERT_TOPIC,
    DEPOSIT_TOPIC, MINT_CONTRACT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC,
    PHOENIX_TOPIC, WITHDRAW_TOPIC,
};
use dusk_core::BlsScalar;
use dusk_data_driver::{
    from_rkyv, json_to_rkyv, json_to_rkyv_pair_u64, json_to_rkyv_u64,
    rkyv_to_json, rkyv_to_json_pair_u64, rkyv_to_json_u64, to_json,
    ConvertibleContract, Error, JsonValue,
};

/// The contract driver for encoding and decoding transactions.
#[derive(Default)]
pub struct ContractDriver;

#[allow(clippy::match_same_arms)]
impl ConvertibleContract for ContractDriver {
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        match fn_name {
            // Transactions
            "convert" => json_to_rkyv::<Withdraw>(json),
            // Protocol calls
            "deposit" => json_to_rkyv_u64(json),
            "mint" | "withdraw" => json_to_rkyv::<Withdraw>(json),
            "mint_to_contract" | "contract_to_contract" => {
                json_to_rkyv::<ContractToContract>(json)
            }
            "contract_to_account" => json_to_rkyv::<ContractToAccount>(json),
            // Queries
            "root" | "num_notes" | "chain_id" => json_to_rkyv::<()>(json),
            "account" => json_to_rkyv::<AccountPublicKey>(json),
            "contract_balance" => json_to_rkyv::<ContractId>(json),
            "opening" => json_to_rkyv_u64(json),
            "existing_nullifiers" => json_to_rkyv::<Vec<BlsScalar>>(json),
            // Feeder Queries
            "leaves_from_height" | "leaves_from_pos" => json_to_rkyv_u64(json),
            "sync"
            | "sync_nullifiers"
            | "sync_contract_balances"
            | "sync_accounts" => json_to_rkyv_pair_u64(json),

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
            // Transactions and internal calls
            "convert" => rkyv_to_json::<Withdraw>(rkyv),
            // Protocol calls
            "deposit" => rkyv_to_json_u64(rkyv),
            "mint" | "withdraw" => rkyv_to_json::<Withdraw>(rkyv),
            "mint_to_contract" | "contract_to_contract" => {
                rkyv_to_json::<ContractToContract>(rkyv)
            }
            "contract_to_account" => rkyv_to_json::<ContractToAccount>(rkyv),
            // Queries
            "root" | "num_notes" | "chain_id" => rkyv_to_json::<()>(rkyv),
            "account" => rkyv_to_json::<AccountPublicKey>(rkyv),
            "contract_balance" => rkyv_to_json::<ContractId>(rkyv),
            "opening" => rkyv_to_json_u64(rkyv),
            "existing_nullifiers" => rkyv_to_json::<Vec<BlsScalar>>(rkyv),
            // Feeder Queries
            "leaves_from_height" | "leaves_from_pos" => rkyv_to_json_u64(rkyv),
            "sync"
            | "sync_nullifiers"
            | "sync_contract_balances"
            | "sync_accounts" => rkyv_to_json_pair_u64(rkyv),

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
            // Transactions and internal calls
            "convert" => Ok(JsonValue::Null),
            // Protocol calls
            "deposit" => Ok(JsonValue::Null),
            "mint" | "withdraw" => Ok(JsonValue::Null),
            "mint_to_contract" | "contract_to_contract" => Ok(JsonValue::Null),
            "contract_to_account" => Ok(JsonValue::Null),
            // Queries
            "root" => rkyv_to_json::<BlsScalar>(rkyv),
            "num_notes" => rkyv_to_json_u64(rkyv),
            "chain_id" => rkyv_to_json::<u8>(rkyv),
            "account" => rkyv_to_json::<AccountData>(rkyv),
            "contract_balance" => rkyv_to_json_u64(rkyv),
            "opening" => {
                Err(Error::Unsupported("opening not supported".into()))
            }
            "existing_nullifiers" => rkyv_to_json::<Vec<BlsScalar>>(rkyv),
            // Feeder Queries
            "sync_accounts" => from_rkyv::<(AccountData, [u8; 193])>(rkyv)
                .and_then(|(data, key)| unsafe {
                    to_json((
                        data,
                        AccountPublicKey::from_slice_unchecked(&key),
                    ))
                    .map_err(Error::from)
                }),
            "sync_nullifiers" => rkyv_to_json::<BlsScalar>(rkyv),
            "sync_contract_balances" => from_rkyv::<(ContractId, u64)>(rkyv)
                .and_then(|(contract, balance)| {
                    to_json((contract, JsonValue::String(balance.to_string())))
                        .map_err(Error::from)
                }),

            "leaves_from_height" | "leaves_from_pos" | "sync" => {
                rkyv_to_json::<NoteLeaf>(rkyv)
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

#[cfg(all(target_family = "wasm", feature = "ffi"))]
dusk_data_driver::generate_wasm_entrypoint!(ContractDriver);
