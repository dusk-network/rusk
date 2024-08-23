// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::transfer::phoenix::Prove;
use rusk_prover::LocalProver;

use super::*;

#[async_trait]
impl HandleRequest for LocalProver {
    fn can_handle(&self, request: &MessageRequest) -> bool {
        matches!(request.event.to_route(), (_, "rusk", topic) | (_, "prover", topic) if topic.starts_with("prove_"))
    }

    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        let topic = request.event.topic.as_str();
        let response = match topic {
            "prove_tx_circuit" => LocalProver::prove(request.event_data())?,
            _ => anyhow::bail!("Unsupported"),
        };
        Ok(ResponseData::new(response))
    }
}
