// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk_prover::{LocalProver, Prover};
use serde::Serialize;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::task;

use rusk_abi::ContractId;

use crate::Rusk;

use super::event::{
    Event, MessageRequest, MessageResponse, RequestData, ResponseData, Target,
};

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";

impl Rusk {
    pub(crate) async fn handle_request(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        match &request.event.to_route() {
            (Target::Contract(_), ..) => {
                let feeder = request.header(RUSK_FEEDER_HEADER).is_some();
                self.handle_contract_query(&request.event, feeder)
            }
            (Target::Host(_), "rusk", "preverify") => {
                self.handle_preverify(request.event_data())
            }
            (Target::Host(_), "rusk", "prove_execute") => {
                Ok(LocalProver.prove_execute(request.event_data())?.into())
            }
            (Target::Host(_), "rusk", "prove_stct") => {
                Ok(LocalProver.prove_stct(request.event_data())?.into())
            }
            (Target::Host(_), "rusk", "prove_stco") => {
                Ok(LocalProver.prove_stco(request.event_data())?.into())
            }
            (Target::Host(_), "rusk", "prove_wfct") => {
                Ok(LocalProver.prove_wfct(request.event_data())?.into())
            }
            (Target::Host(_), "rusk", "prove_wfco") => {
                Ok(LocalProver.prove_wfco(request.event_data())?.into())
            }

            (Target::Host(_), "rusk", "provisioners") => {
                self.get_provisioners()
            }
            (Target::Host(_), "rusk", "crs") => self.get_crs(),
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }

    fn handle_contract_query(
        &self,
        event: &Event,
        feeder: bool,
    ) -> anyhow::Result<ResponseData> {
        let contract = event.target.inner();
        let contract_bytes = hex::decode(contract)?;

        let contract_bytes = contract_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid contract bytes"))?;

        if feeder {
            let (sender, receiver) = mpsc::channel();

            let rusk = self.clone();
            let topic = event.topic.clone();
            let arg = event.data.as_bytes().to_vec();

            thread::spawn(move || {
                rusk.feeder_query_raw(
                    ContractId::from_bytes(contract_bytes),
                    topic,
                    arg,
                    sender,
                );
            });
            Ok(ResponseData::Channel(receiver))
        } else {
            let data = self
                .query_raw(
                    ContractId::from_bytes(contract_bytes),
                    event.topic.clone(),
                    event.data.as_bytes(),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(data.into())
        }
    }

    fn handle_preverify(&self, data: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = phoenix_core::Transaction::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?;
        self.preverify(&tx.into())?;
        Ok(ResponseData::None)
    }

    fn get_provisioners(&self) -> anyhow::Result<ResponseData> {
        let prov: Vec<_> = self
            .provisioners()
            .unwrap()
            .iter()
            .filter_map(|(key, stake)| {
                let key = bs58::encode(key.to_bytes()).into_string();
                let (amount, eligibility) = stake.amount.unwrap_or_default();
                (amount > 0).then_some(Provisioner {
                    amount,
                    eligibility,
                    key,
                })
            })
            .collect();

        Ok(serde_json::to_value(prov)?.into())
    }

    fn get_crs(&self) -> anyhow::Result<ResponseData> {
        let crs = rusk_profile::get_common_reference_string()?;
        Ok(crs.into())
    }
}

#[derive(Serialize)]
struct Provisioner {
    key: String,
    amount: u64,
    eligibility: u64,
}
