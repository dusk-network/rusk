// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::aggregator::Aggregator;
use crate::commons::{spawn_send_reduction, Block, ConsensusError};
use crate::execution_ctx::ExecutionCtx;
use crate::firststep::handler;
use crate::messages::{Message, Payload};
use crate::user::committee::Committee;
use std::ops::Deref;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    pub timeout: u16,
    handler: handler::Reduction,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
            timeout: 0,
            handler: handler::Reduction {
                aggr: Aggregator::default(),
                candidate: Block::default(),
            },
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
                committee.get_my_pubkey(),
                ctx.round_update,
                ctx.step,
                ctx.outbound.clone(),
                ctx.inbound.clone(),
            );
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler) {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler).await
    }

    pub fn name(&self) -> &'static str {
        "1th_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
