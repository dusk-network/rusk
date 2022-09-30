// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{spawn_send_reduction, Block, ConsensusError};
use crate::execution_ctx::ExecutionCtx;
use crate::messages::{Message, Payload};
use crate::secondstep::handler;
use crate::user::committee::Committee;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    handler: handler::Reduction,
    candidate: Option<Block>,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
            handler: handler::Reduction {
                aggr: Default::default(),
                first_step_votes: Default::default(),
            },
            candidate: None,
        }
    }

    pub fn initialize(&mut self, msg: &Message) {
        self.candidate = None;
        self.handler.first_step_votes = Default::default();

        if let Payload::StepVotesWithCandidate(p) = msg.payload.clone() {
            self.handler.first_step_votes = p.sv;
            self.candidate = Some(p.candidate);
        }
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        if committee.am_member() {
            //  Send reduction in async way
            if let Some(b) = &self.candidate {
                spawn_send_reduction(
                    b.clone(),
                    committee.get_my_pubkey(),
                    ctx.round_update,
                    ctx.step,
                    ctx.outbound.clone(),
                    ctx.inbound.clone(),
                );
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler).await {
            return Ok(m);
        }

        // TODO: Handle  Err(SelectError::Timeout)
        // TODO: create agreement with empty block self.handler.on_timeout()
        ctx.event_loop(&committee, &mut self.handler).await
    }

    pub fn name(&self) -> &'static str {
        "2nd_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
