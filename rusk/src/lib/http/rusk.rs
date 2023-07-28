// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use rusk_abi::ContractId;

use crate::Rusk;

use super::event::{DataType, Request, Response, Target};

impl Rusk {
    pub(crate) async fn handle_request(&self, request: Request) -> Response {
        match &request.target {
            Target::Contract(contract) => {
                let contract_bytes = hex::decode(contract);
                if let Err(e) = &contract_bytes {
                    return Response {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    };
                };
                let contract_bytes =
                    contract_bytes.expect("to be already checked").try_into();
                if let Err(e) = &contract_bytes {
                    return Response {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: "Invalid contract bytes".to_string().into(),
                    };
                };
                let response = self.query_raw(
                    ContractId::from_bytes(
                        contract_bytes.expect("to be valid"),
                    ),
                    request.topic.clone(),
                    request.data.as_bytes(),
                );
                match response {
                    Err(e) => Response {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    },
                    Ok(data) => Response {
                        data: data.into(),
                        headers: request.x_headers(),
                        error: None,
                    },
                }
            }
            _ => Response {
                data: DataType::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }
}
