// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{spawn_send_reduction, Block, ConsensusError};
use crate::config;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::{Message, Payload};
use crate::secondstep::handler;
use crate::user::committee::Committee;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction<T> {
    handler: handler::Reduction,
    candidate: Option<Block>,
    timeout_millis: u64,
    executor: Arc<Mutex<T>>,
}

impl<T: Operations + 'static> Reduction<T> {
    pub fn new(executor: Arc<Mutex<T>>) -> Self {
        Self {
            handler: handler::Reduction {
                aggr: Default::default(),
                first_step_votes: Default::default(),
            },
            candidate: None,
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            executor,
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
                    committee.get_my_pubkey().clone(),
                    ctx.round_update.clone(),
                    ctx.step,
                    ctx.outbound.clone(),
                    ctx.inbound.clone(),
                    self.executor.clone(),
                );
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
        "2nd_reduction"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
