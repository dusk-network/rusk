// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use rusk_abi::ContractId;

use crate::http::{DataType, WsRequest, WsResponse};
use crate::Rusk;

use super::event::WsTarget;

impl Rusk {
    pub(crate) async fn handle_request(
        &self,
        request: WsRequest,
    ) -> WsResponse {
        match &request.target {
            WsTarget::Contract(contract) => {
                let contract_bytes = hex::decode(contract);
                if let Err(e) = &contract_bytes {
                    return WsResponse {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    };
                };
                let contract_bytes =
                    contract_bytes.expect("to be already checked").try_into();
                if let Err(e) = &contract_bytes {
                    return WsResponse {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: "Invalid contract bytes".to_string().into(),
                    };
                };
                // .map_err(|e| anyhow::anyhow!("Invalid contract specified"))
                // contract_bytes.and_then(|contract_bytes| {
                let response = self.query_raw(
                    ContractId::from_bytes(
                        contract_bytes.expect("to be valid"),
                    ),
                    request.topic.clone(),
                    request.data.as_bytes(),
                );
                // .map_err(|e| anyhow::anyhow!(e.to_string()))
                // });

                match response {
                    Err(e) => WsResponse {
                        data: DataType::None,
                        headers: request.x_headers(),
                        error: format!("{e}").into(),
                    },
                    Ok(data) => WsResponse {
                        data: data.into(),
                        headers: request.x_headers(),
                        error: None,
                    },
                }
            }
            _ => WsResponse {
                data: DataType::None,
                headers: request.x_headers(),
                error: Some("Unsupported".into()),
            },
        }
    }
}
