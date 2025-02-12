// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::StakeFundOwner;
use dusk_core::transfer::Transaction;
use dusk_vm::{execute, ExecutionConfig};
use node::vm::VMExecution;
use rusk_profile::CRS_17_HASH;
use serde::Serialize;
use serde_json::json;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::task;

use crate::node::Rusk;

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";

#[async_trait]
impl HandleRequest for Rusk {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("contracts", Some(_), _) => true,
            ("node", _, "provisioners") => true,
            ("account", Some(_), "status") => true,
            ("node", _, "crs") => true,
            ("transactions", _, "simulate") => true,
            _ => false,
        }
    }

    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData> {
        match request.uri.inner() {
            ("contracts", Some(contract_id), method) => {
                let feeder = request.header(RUSK_FEEDER_HEADER).is_some();
                let data = request.data.as_bytes();
                self.handle_contract_query(contract_id, method, data, feeder)
            }
            ("node", _, "provisioners") => self.get_provisioners(),
            ("account", Some(pk), "status") => self.get_account(pk),
            ("node", _, "crs") => self.get_crs(),
            ("transactions", _, "simulate") => {
                let tx = request.data.as_string();
                self.simulate_tx(&tx)
            }
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }
}

impl Rusk {
    fn handle_contract_query(
        &self,
        contract: &str,
        topic: &str,
        data: &[u8],
        feeder: bool,
    ) -> anyhow::Result<ResponseData> {
        let contract_bytes = hex::decode(contract)?;

        let contract_bytes = contract_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid contract bytes"))?;
        let contract_id = ContractId::from_bytes(contract_bytes);
        let fn_name = topic.to_string();
        let data = data.to_vec();
        if feeder {
            let (sender, receiver) = mpsc::channel();

            let rusk = self.clone();

            thread::spawn(move || {
                rusk.feeder_query_raw(contract_id, fn_name, data, sender);
            });
            Ok(ResponseData::new(receiver))
        } else {
            let data = self
                .query_raw(contract_id, fn_name, data)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(ResponseData::new(data))
        }
    }

    fn get_provisioners(&self) -> anyhow::Result<ResponseData> {
        let prov: Vec<_> = self
            .provisioners(None)
            .expect("Cannot query state for provisioners")
            .map(|(key, stake)| {
                let owner = StakeOwner::from(&key.owner);
                let key = bs58::encode(key.account.to_bytes()).into_string();
                let amount = stake.amount.unwrap_or_default();

                Provisioner {
                    amount: amount.value,
                    locked_amt: amount.locked,
                    eligibility: amount.eligibility,
                    key,
                    reward: stake.reward,
                    faults: stake.faults,
                    hard_faults: stake.hard_faults,
                    owner,
                }
            })
            .collect();

        Ok(ResponseData::new(serde_json::to_value(prov)?))
    }

    fn get_account(&self, pk: &str) -> anyhow::Result<ResponseData> {
        let pk = bs58::decode(pk)
            .into_vec()
            .map_err(|_| anyhow::anyhow!("Invalid bs58 account"))?;
        let pk = BlsPublicKey::from_slice(&pk)
            .map_err(|_| anyhow::anyhow!("Invalid bls account"))?;
        let account = self
            .account(&pk)
            .map(|account| {
                json!({
                    "balance": account.balance,
                    "nonce": account.nonce,
                })
            })
            .map_err(|e| anyhow::anyhow!("Cannot query the state {e:?}"))?;
        Ok(ResponseData::new(account))
    }

    fn get_crs(&self) -> anyhow::Result<ResponseData> {
        let crs = rusk_profile::get_common_reference_string()?;
        Ok(ResponseData::new(crs).with_header("crs-hash", CRS_17_HASH))
    }

    fn simulate_tx(&self, tx: &str) -> anyhow::Result<ResponseData> {
        let tx = hex::decode(tx).map_err(|e| {
            anyhow::anyhow!("Failed to decode transaction: {e}")
        })?;
        let tx = Transaction::from_slice(&tx)
            .map_err(|e| anyhow::anyhow!("Invalid Data: {e:?}"))?;
        if tx.gas_limit() > self.get_block_gas_limit() {
            return Err(anyhow::anyhow!("Gas limit is too high."));
        }
        let mut session = self.query_session(None).map_err(|e| {
            anyhow::anyhow!("Failed to initialize a session: {e:?}")
        })?;
        let config = ExecutionConfig {
            gas_per_deploy_byte: self.gas_per_deploy_byte(),
            min_deploy_gas_price: self.min_deployment_gas_price(),
            min_deploy_points: self.min_deploy_points(),
            with_public_sender: false,
        };
        let receipt = execute(&mut session, &tx, &config)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(ResponseData::new(json!({
            "gas-limit": receipt.gas_limit,
            "gas-spent": receipt.gas_spent,
            "data": receipt.data.map_err(|e| anyhow::anyhow!("Contract terminated with error: {e:?}"))?,
        })))
    }
}

#[derive(Serialize)]
struct Provisioner {
    key: String,
    amount: u64,
    locked_amt: u64,
    eligibility: u64,
    reward: u64,
    faults: u8,
    hard_faults: u8,
    owner: StakeOwner,
}

#[derive(Serialize)]
enum StakeOwner {
    Account(String),
    Contract(String),
}

impl From<&StakeFundOwner> for StakeOwner {
    fn from(value: &StakeFundOwner) -> Self {
        match value {
            StakeFundOwner::Account(account) => StakeOwner::Account(
                bs58::encode(account.to_bytes()).into_string(),
            ),
            StakeFundOwner::Contract(contract) => {
                StakeOwner::Contract(hex::encode(contract.as_bytes()))
            }
        }
    }
}
