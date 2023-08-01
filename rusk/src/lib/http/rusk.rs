// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::vm::VMExecution;
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
            let arg = event.data.as_bytes();

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
}
