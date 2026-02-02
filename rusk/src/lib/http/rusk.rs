// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use crate::BlsPublicKey;
use anyhow::anyhow;
use dusk_bytes::{DeserializableSlice, ParseHexStr, Serializable};
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::Signature;
use dusk_core::stake::{StakeFundOwner, STAKE_CONTRACT};
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_data_driver::ConvertibleContract;
use event::RequestData;
use rusk_profile::CRS_17_HASH;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::sync::mpsc;
use std::thread;

use crate::node::Rusk;

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";
const UPLOAD_DRIVER_RESPONSE: &str = "driver upload ok";

#[derive(Debug, Serialize, Deserialize)]
struct ContractMetadataResponse {
    owner: String,
    driver_available: bool,
    driver_signature: Option<String>,
    created_at: Option<String>,
}

#[async_trait]
impl HandleRequest for Rusk {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("contracts", Some(_), _) => true,
            ("driver", Some(_), _) => true,
            ("contract_owner", Some(_), _) => true,
            ("contract", Some(_), "upload_driver") => true,
            ("contract", Some(_), "download_driver") => true,
            ("contract", Some(_), "metadata") => true,
            ("node", _, "provisioners") => true,
            ("node", _, "crs") => true,
            _ => false,
        }
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
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
                self.handle_data_driver(contract_id, method, &request.data)
            }
            ("contract_owner", Some(contract_id), _method) => {
                self.get_contract_owner(contract_id)
            }
            ("contract", Some(contract_id), "upload_driver") => {
                let sign = request
                    .header("sign")
                    .and_then(|v| v.as_str())
                    .ok_or(HttpError::invalid_input("Signature missing"))?;
                self.upload_driver(contract_id, sign, request.data.as_bytes())
            }
            ("contract", Some(contract_id), "download_driver") => {
                self.download_driver(contract_id)
            }
            ("contract", Some(contract_id), "metadata") => {
                self.metadata(contract_id)
            }
            ("node", _, "provisioners") => Ok(self.get_provisioners()?),
            ("node", _, "crs") => Ok(self.get_crs()?),
            _ => Err(HttpError::Unsupported),
        }
    }
}

impl Rusk {
    fn data_driver<C: TryInto<ContractId>>(
        &self,
        contract_id: C,
    ) -> anyhow::Result<Option<Box<dyn ConvertibleContract>>> {
        let contract_id = contract_id
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid contractId"))?;

        Ok(match contract_id {
            TRANSFER_CONTRACT => {
                Some(Box::new(dusk_transfer_contract_dd::ContractDriver))
            }
            STAKE_CONTRACT => {
                Some(Box::new(dusk_stake_contract_dd::ContractDriver))
            }
            _ => self.get_driver_executor(&contract_id)?,
        })
    }

    fn get_driver_executor(
        &self,
        contract_id: &ContractId,
    ) -> anyhow::Result<Option<Box<dyn ConvertibleContract>>> {
        let cached_instance = {
            let instance_cache = self.instance_cache.read();
            instance_cache.get(contract_id).cloned()
        };
        Ok(match cached_instance {
            Some(driver_executor) => Some(Box::new(driver_executor.clone())),
            _ => {
                let driver_store = self.driver_store.read();
                match driver_store.get_bytecode(contract_id)? {
                    Some(bytecode) => {
                        let driver_executor = DriverExecutor::from_bytecode(
                            contract_id,
                            bytecode,
                        )?;
                        let mut instance_cache = self.instance_cache.write();
                        instance_cache
                            .insert(*contract_id, driver_executor.clone());
                        Some(Box::new(driver_executor))
                    }
                    _ => None,
                }
            }
        })
    }

    fn handle_data_driver(
        &self,
        contract_id: &str,
        method: &str,
        data: &RequestData,
    ) -> HttpResult<ResponseData> {
        let (method, target) = method.split_once(':').unwrap_or((method, ""));
        let driver = self
            .data_driver(contract_id.to_string())?
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
            method => {
                return Err(HttpError::generic(format!(
                    "Unsupported data driver method {method}"
                )))
            }
        };
        Ok(result)
    }

    fn get_contract_owner(
        &self,
        contract_id: &str,
    ) -> HttpResult<ResponseData> {
        let contract_id = ContractId::try_from(contract_id.to_string())
            .map_err(|_| HttpError::invalid_input("Invalid contract id"))?;
        self.query_metadata(&contract_id)
            .map(|metadata| ResponseData::new(metadata.owner))
            .map_err(|e| {
                HttpError::generic(format!("Contract owner not found: {e}"))
            })
    }

    fn upload_driver(
        &self,
        contract_id: &str,
        sig: impl AsRef<str>,
        data: &[u8],
    ) -> HttpResult<ResponseData> {
        let contract_id = ContractId::try_from(contract_id.to_string())
            .map_err(|_| HttpError::invalid_input("Invalid contract id"))?;

        // compute hash
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let hashed_data = hasher.finalize();
        let hash = hashed_data.to_vec();

        // verify owner's signature
        let owner = self
            .query_metadata(&contract_id)
            .map(|m| m.owner)
            .map_err(|e| anyhow::anyhow!("Contract owner not found: {e}"))?;
        let pk = BlsPublicKey::from_slice(&owner).map_err(|e| {
            anyhow::anyhow!("Invalid owner public key found: {e:?}")
        })?;
        let signature = Signature::from_hex_str(sig.as_ref())
            .map_err(|e| anyhow::anyhow!("Invalid signature: {e}"))?;
        pk.verify(&signature, &hash).map_err(|e| {
            anyhow::anyhow!("Signature verification failed: {e}")
        })?;

        let mut driver_store = self.driver_store.write();
        driver_store
            .store_bytecode_and_signature(
                &contract_id,
                data,
                signature.to_bytes(),
            )
            .map_err(|e| {
                anyhow::anyhow!("Cannot store bytecode and signature: {e:?}")
            })?;
        let mut instance_cache = self.instance_cache.write();
        instance_cache.remove(&contract_id);
        Ok(ResponseData::new(UPLOAD_DRIVER_RESPONSE.to_string()))
    }

    fn download_driver(&self, contract_id: &str) -> HttpResult<ResponseData> {
        let contract_id = ContractId::try_from(contract_id.to_string())
            .map_err(|_| HttpError::invalid_input("Invalid contract id"))?;
        let driver_store = self.driver_store.read();
        let driver_bytecode = driver_store
            .get_bytecode(&contract_id)
            .map_err(|_| anyhow::anyhow!("Driver not registered"))?
            .ok_or_else(|| anyhow::anyhow!("Driver not found"))?;
        Ok(ResponseData::new(driver_bytecode)
            .with_force_binary(true)
            .with_header("content-type", "application/wasm"))
    }

    fn metadata(&self, contract_id: &str) -> HttpResult<ResponseData> {
        let contract_id = ContractId::try_from(contract_id.to_string())
            .map_err(|_| anyhow::anyhow!("Invalid contract id"))?;
        let owner = self
            .query_metadata(&contract_id)
            .map(|metadata| metadata.owner)
            .unwrap_or_default();
        let driver_store = self.driver_store.read();
        let driver_signature =
            driver_store.get_signature(&contract_id).unwrap_or(None);
        let driver_available = driver_store.driver_available(&contract_id);
        let response = ContractMetadataResponse {
            owner: bs58::encode(&owner).into_string(),
            driver_available,
            driver_signature: driver_signature.map(hex::encode),
            created_at: None,
        };
        let response_value = serde_json::to_value(&response)
            .map_err(|_| anyhow::anyhow!("Metadata conversion error"))?;
        Ok(ResponseData::new(DataType::Json(response_value)))
    }

    fn handle_contract_query(
        &self,
        contract: &str,
        fn_name: &str,
        data: &RequestData,
        feeder: bool,
        json: bool,
    ) -> HttpResult<ResponseData> {
        let contract_id = ContractId::try_from(contract.to_string())
            .map_err(|_| HttpError::invalid_input("Invalid contract bytes"))?;

        let mut driver = None;

        let call_arg = if json {
            let json = data.as_string();
            driver = self.data_driver(contract_id)?;
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
