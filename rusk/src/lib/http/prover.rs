// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_prover::{LocalProver, Prover};

use super::*;

#[async_trait]
impl HandleRequest for LocalProver {
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        let topic = request.event.topic.as_str();
        let response = match topic {
            "prove_execute" => self.prove_execute(request.event_data())?,
            "prove_stct" => self.prove_stct(request.event_data())?,
            "prove_wfct" => self.prove_wfct(request.event_data())?,
            _ => anyhow::bail!("Unsupported"),
        };
        Ok(ResponseData::new(response))
    }
}
