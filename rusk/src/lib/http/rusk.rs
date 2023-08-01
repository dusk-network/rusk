// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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
        request: MessageRequest,
    ) -> MessageResponse {
        let data = match &request.event.target {
            Target::Contract(contract) => self.handle_contract_query(
                &request.event,
                request.header(RUSK_FEEDER_HEADER).is_some(),
            ),
            // Target::Host(target) if target == "rusk" => {
            //     match &request.event.topic {
            //         "preverify" => {
            //             self.handle_preverify(request.event.data.as_bytes())
            //         }
            //         _ => Err(anyhow::anyhow!("Unsupported")),
            //     }
            // }
            _ => Err(anyhow::anyhow!("Unsupported")),
        };

        data.map(|data| MessageResponse {
            data,
            error: None,
            headers: request.x_headers(),
        })
        .unwrap_or_else(|e| request.to_error(e.to_string()))
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

        match feeder {
            true => {
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
            }
            false => {
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
}
