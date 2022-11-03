// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{spawn_send_reduction, ConsensusError};
use crate::config;
use crate::execution_ctx::ExecutionCtx;
use crate::firststep::handler;
use crate::messages::{Message, Payload};
use crate::user::committee::Committee;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    timeout_millis: u64,
    handler: handler::Reduction,
    executor: Arc<Mutex<dyn crate::contract_state::Operations>>,
}

impl Reduction {
    pub fn new(executor: Arc<Mutex<dyn crate::contract_state::Operations>>) -> Self {
        Self {
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            handler: handler::Reduction::default(),
            executor,
        }
    }

    pub fn initialize(&mut self, msg: &Message) {
        if let Payload::NewBlock(p) = msg.clone().payload {
            self.handler.candidate = p.deref().candidate.clone();
        }
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        if committee.am_member() {
            // Send reduction async
            spawn_send_reduction(
                self.handler.candidate.clone(),
                committee.get_my_pubkey().clone(),
                ctx.round_update.clone(),
                ctx.step,
                ctx.outbound.clone(),
                ctx.inbound.clone(),
                self.executor.clone(),
            );
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler).await {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler, &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "1th_reduction"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }
    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
