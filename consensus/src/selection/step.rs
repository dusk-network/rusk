// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::ConsensusError;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::Message;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config;
use crate::selection::block_generator::Generator;
use crate::selection::handler;
use crate::user::committee::Committee;
use tracing::error;

pub const COMMITTEE_SIZE: usize = 1;

pub struct Selection<T>
where
    T: Operations,
{
    handler: handler::Selection,
    bg: Generator<T>,
    timeout_millis: u64,
}

impl<T: Operations> Selection<T> {
    pub fn new(executor: Arc<Mutex<T>>) -> Self {
        Self {
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            handler: handler::Selection {},
            bg: Generator::new(executor),
        }
    }

    pub fn initialize(&mut self, _msg: &Message) {}

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        if committee.am_member() {
            if let Ok(msg) = self
                .bg
                .generate_candidate_message(&ctx.round_update, ctx.step)
                .await
            {
                // Broadcast the candidate block for this round/iteration.
                if let Err(e) = ctx.outbound.send(msg.clone()).await {
                    error!("could not send newblock msg due to {:?}", e);
                }

                // register new candidate in local state
                match self
                    .handler
                    .collect(msg, &ctx.round_update, ctx.step, &committee)
                {
                    Ok(f) => {
                        if let HandleMsgOutput::FinalResult(msg) = f {
                            return Ok(msg);
                        }
                    }
                    Err(e) => error!("invalid candidate generated due to {:?}", e),
                };
            } else {
                error!("block generator couldn't create candidate block")
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler).await {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler, &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "selection"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }
    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
