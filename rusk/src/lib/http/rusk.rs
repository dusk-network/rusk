// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use anyhow::anyhow;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{StakeFundOwner, STAKE_CONTRACT};
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_data_driver::ConvertibleContract;
use event::RequestData;
use rusk_profile::CRS_17_HASH;
use serde::Serialize;
use serde_json::json;
use std::sync::mpsc;
use std::thread;

use crate::node::Rusk;

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";

#[async_trait]
impl HandleRequest for Rusk {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("contracts", Some(_), _) => true,
            ("driver", Some(_), _) => true,
            ("node", _, "provisioners") => true,
            ("account", Some(_), "status") => true,
            ("node", _, "crs") => true,
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
                self.handle_contract_query(
                    contract_id,
                    method,
                    &request.data,
                    feeder,
                    request.is_json(),
                )
            }
            ("driver", Some(contract_id), method) => {
                Self::handle_data_driver(contract_id, method, &request.data)
            }
            ("node", _, "provisioners") => self.get_provisioners(),

            ("account", Some(pk), "status") => self.get_account(pk),
            ("node", _, "crs") => self.get_crs(),
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }
}

impl Rusk {
    fn data_driver<C: TryInto<ContractId>>(
        contract_id: C,
    ) -> anyhow::Result<Option<Box<dyn ConvertibleContract>>> {
        let contract_id = contract_id
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid contractId"))?;

        let driver: Option<Box<dyn ConvertibleContract>> = match contract_id {
            TRANSFER_CONTRACT => {
                Some(Box::new(dusk_transfer_contract_dd::ContractDriver))
            }
            STAKE_CONTRACT => {
                Some(Box::new(dusk_stake_contract_dd::ContractDriver))
            }
            _ => None,
        };
        Ok(driver)
    }

    fn handle_data_driver(
        contract_id: &str,
        method: &str,
        data: &RequestData,
    ) -> anyhow::Result<ResponseData> {
        let (method, target) = method.split_once(':').unwrap_or((method, ""));
        let driver = Self::data_driver(contract_id.to_string())?
            .ok_or(anyhow::anyhow!("Unsupported contractId {contract_id}"))?;
        let result = match method {
            "decode_event" => ResponseData::new(
                driver
                    .decode_event(target, data.as_bytes())
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
            "decode_input_fn" => ResponseData::new(
                driver
                    .decode_input_fn(target, data.as_bytes())
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
            "decode_output_fn" => ResponseData::new(
                driver
                    .decode_output_fn(target, data.as_bytes())
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
            "encode_input_fn" => ResponseData::new(
                driver
                    .encode_input_fn(target, &data.as_string())
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
            "get_schema" => ResponseData::new(driver.get_schema().to_string()),
            "get_version" => {
                ResponseData::new(driver.get_version().to_string())
            }
            method => anyhow::bail!("Unsupported datadriver method {method}"),
        };
        Ok(result)
    }

    fn handle_contract_query(
        &self,
        contract: &str,
        fn_name: &str,
        data: &RequestData,
        feeder: bool,
        json: bool,
    ) -> anyhow::Result<ResponseData> {
        let contract_id = ContractId::try_from(contract.to_string())
            .map_err(|_| anyhow::anyhow!("Invalid contract bytes"))?;

        let mut driver = None;

        let call_arg = if json {
            let json = data.as_string();
            driver = Self::data_driver(contract_id)?;
            driver
                .as_ref()
                .ok_or(anyhow::anyhow!("Unsupported contract {contract}"))?
                .encode_input_fn(fn_name, &json)
                .map_err(|e| anyhow::anyhow!("InvalidJson {e:?}"))?
        } else {
            data.as_bytes().to_vec()
        };

        let fn_name = fn_name.to_string();
        if feeder {
            let (sender, receiver) = mpsc::channel();

            let rusk = self.clone();
            let fn_name_feeder = fn_name.clone();

            thread::spawn(move || {
                let _ = rusk.feeder_query_raw(
                    contract_id,
                    fn_name_feeder,
                    call_arg,
                    sender,
                );
            });

            if let Some(driver) = driver {
                let (json_sender, json_receiver) = mpsc::channel();
                thread::spawn(move || {
                    let mut first = true;
                    json_sender.send("[".as_bytes().to_vec())?;
                    while let Some(raw_output) = receiver.iter().next() {
                        let json = driver
                            .decode_output_fn(&fn_name, &raw_output)
                            .map_err(|e| anyhow!("cannot decode {e}"))?;
                        if first {
                            first = false;
                        } else {
                            json_sender.send(",".as_bytes().to_vec())?;
                        }
                        json_sender
                            .send(json.to_string().as_bytes().to_vec())?;
                    }
                    json_sender.send("]".as_bytes().to_vec())?;
                    anyhow::Ok(())
                });
                Ok(ResponseData::new(DataType::JsonChannel(json_receiver)))
            } else {
                Ok(ResponseData::new(receiver))
            }
        } else {
            let raw_output = self
                .query_raw(contract_id, &fn_name, call_arg)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let response = if let Some(driver) = driver {
                match driver.decode_output_fn(&fn_name, &raw_output) {
                    Ok(json) => ResponseData::new(json),
                    Err(_) => ResponseData::new(raw_output),
                }
            } else {
                ResponseData::new(raw_output)
            };
            Ok(response)
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
