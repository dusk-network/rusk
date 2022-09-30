// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::ConsensusError;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::Message;
use crate::msg_handler::MsgHandler;

use crate::selection::block_generator::Generator;
use crate::selection::handler;
use crate::user::committee::Committee;
use tracing::error;

pub const COMMITTEE_SIZE: usize = 1;

pub struct Selection {
    handler: handler::Selection,
    bg: Generator,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            handler: handler::Selection {},
            bg: Generator {},
        }
    }

    pub fn initialize(&mut self, _msg: &Message) {}

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        if committee.am_member() {
            let msg = self
                .bg
                .generate_candidate_message(ctx.round_update, ctx.step)
                .await;

            // Broadcast the candidate block for this round/iteration.
            if let Err(e) = ctx.outbound.send(msg.clone()).await {
                error!("could not send newblock msg due to {:?}", e);
            }

            // register new candidate in local state
            match self
                .handler
                .handle(msg, ctx.round_update, ctx.step, &committee)
            {
                Ok(f) => return Ok(f.0),
                Err(e) => error!("invalid candidate generated due to {:?}", e),
            };
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler).await {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler).await
    }

    pub fn name(&self) -> &'static str {
        "selection"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
