// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::event::Event;
use super::*;

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk_profile::CRS_17_HASH;
use serde::Serialize;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::task;
use tungstenite::http::request;

use execution_core::ContractId;

use crate::node::Rusk;

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";

#[async_trait]
impl HandleRequest for Rusk {
    fn can_handle(&self, request: &MessageRequest) -> bool {
        matches!(
            &request.event.to_route(),
            (Target::Contract(_), ..) | (Target::Host(_), "rusk", _)
        )
    }
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match request.uri.inner() {
            ("contracts", Some(_), _) => true,
            ("transactions", _, "preverify") => true,
            ("node", _, "provisioners") => true,
            ("node", _, "crs") => true,
            _ => false,
        }
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData> {
        info!("received event {request:?}");
        match request.uri.inner() {
            ("contracts", Some(contract_id), method) => {
                let feeder = request.header(RUSK_FEEDER_HEADER).is_some();
                let data = request.data.as_bytes();
                self.handle_contract_query(contract_id, method, data, feeder)
            }
            ("transactions", _, "preverify") => {
                self.handle_preverify(request.data.as_bytes())
            }
            ("node", _, "provisioners") => self.get_provisioners(),
            ("node", _, "crs") => self.get_crs(),
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }

    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        match &request.event.to_route() {
            (Target::Contract(_), ..) => {
                let feeder = request.header(RUSK_FEEDER_HEADER).is_some();
                self.handle_contract_query_legacy(&request.event, feeder)
            }
            (Target::Host(_), "rusk", "preverify") => {
                self.handle_preverify(request.event_data())
            }
            (Target::Host(_), "rusk", "provisioners") => {
                self.get_provisioners()
            }
            (Target::Host(_), "rusk", "crs") => self.get_crs(),
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }
}

impl Rusk {
    fn handle_contract_query_legacy(
        &self,
        event: &Event,
        feeder: bool,
    ) -> anyhow::Result<ResponseData> {
        let contract = event.target.inner();
        let topic = &event.topic;
        let data = event.data.as_bytes();

        self.handle_contract_query(contract, topic, data, feeder)
    }
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

    fn handle_preverify(&self, data: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = execution_core::transfer::Transaction::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?;
        self.preverify(&tx.into())?;
        Ok(ResponseData::new(DataType::None))
    }

    fn get_provisioners(&self) -> anyhow::Result<ResponseData> {
        let prov: Vec<_> = self
            .provisioners(None)
            .expect("Cannot query state for provisioners")
            .map(|(key, stake)| {
                let key = bs58::encode(key.account.to_bytes()).into_string();
                let amount = stake.amount.unwrap_or_default();

                Provisioner {
                    amount: amount.value,
                    eligibility: amount.eligibility,
                    key,
                    reward: stake.reward,
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
    eligibility: u64,
    reward: u64,
}
