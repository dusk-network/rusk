// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::anyhow;

use dusk_core::transfer::phoenix::Prove;
use rusk_prover::LocalProver;

use super::*;

#[async_trait]
impl HandleRequest for LocalProver {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        matches!(request.uri.inner(), ("prover", _, "prove"))
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData> {
        let data = request.data.as_bytes();
        let response = match request.uri.inner() {
            ("prover", _, "prove") => {
                LocalProver.prove(data).map_err(|e| anyhow!(e))?
            }
            _ => anyhow::bail!("Unsupported"),
        };
        Ok(ResponseData::new(response))
    }
}
