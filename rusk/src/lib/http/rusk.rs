// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::{mpsc, Arc};
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
        match &request.event.target {
            Target::Contract(contract) => {
                let contract_bytes = hex::decode(contract);
                if let Err(e) = &contract_bytes {
                    return MessageResponse {
                        data: ResponseData::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    };
                };
                let contract_bytes =
                    contract_bytes.expect("to be already checked").try_into();
                if let Err(e) = &contract_bytes {
                    return MessageResponse {
                        data: ResponseData::None,
                        headers: request.x_headers(),
                        error: "Invalid contract bytes".to_string().into(),
                    };
                };

                match request.header(RUSK_FEEDER_HEADER).is_some() {
                    true => {
                        let (sender, receiver) = mpsc::channel();

                        let rusk = self.clone();
                        let topic = request.event.topic.clone();
                        let arg = request.event.data.as_bytes();

                        task::spawn(async move {
                            rusk.feeder_query_raw(
                                ContractId::from_bytes(
                                    contract_bytes.expect("to be valid"),
                                ),
                                topic,
                                arg,
                                sender,
                            );
                        });

                        MessageResponse {
                            data: ResponseData::Channel(receiver),
                            headers: request.x_headers(),
                            error: None,
                        }
                    }
                    false => {
                        match self.query_raw(
                            ContractId::from_bytes(
                                contract_bytes.expect("to be valid"),
                            ),
                            request.event.topic.clone(),
                            request.event.data.as_bytes(),
                        ) {
                            Ok(data) => MessageResponse {
                                data: data.into(),
                                headers: request.x_headers(),
                                error: None,
                            },
                            Err(e) => MessageResponse {
                                data: ResponseData::None,
                                headers: request.x_headers(),
                                error: format!("{e}").into(),
                            },
                        }
                    }
                }
            }
            _ => MessageResponse {
                data: ResponseData::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }
}
