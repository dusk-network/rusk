// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk_profile::CRS_17_HASH;
use rusk_prover::{LocalProver, Prover};
use serde::Serialize;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::task;

use rusk_abi::ContractId;

use crate::Rusk;

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
            "prove_stco" => self.prove_stco(request.event_data())?,
            "prove_wfct" => self.prove_wfct(request.event_data())?,
            "prove_wfco" => self.prove_wfco(request.event_data())?,
            _ => anyhow::bail!("Unsupported"),
        };
        Ok(ResponseData::new(response))
    }
}
