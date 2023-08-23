// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{spawn_send_reduction, ConsensusError, Database};
use crate::config;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use crate::firststep::handler;
use crate::user::committee::Committee;
use node_data::ledger::to_str;
use node_data::message::{Message, Payload};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

#[allow(unused)]
pub struct Reduction<T, DB: Database> {
    timeout_millis: u64,
    handler: handler::Reduction<DB>,
    executor: Arc<Mutex<T>>,
}

impl<T: Operations + 'static, DB: Database> Reduction<T, DB> {
    pub fn new(executor: Arc<Mutex<T>>, db: Arc<Mutex<DB>>) -> Self {
        Self {
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            handler: handler::Reduction::new(db),
            executor,
        }
    }

    pub fn reinitialize(&mut self, msg: &Message, round: u64, step: u8) {
        self.handler.reset();

        if let Payload::NewBlock(p) = msg.clone().payload {
            self.handler.candidate = p.deref().candidate.clone();
        }

        debug!(
            event = "init",
            name = self.name(),
            round = round,
            step = step,
            timeout = self.timeout_millis,
            hash = to_str(&self.handler.candidate.header.hash),
        )
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        if committee.am_member() {
            // Send reduction async
            spawn_send_reduction(
                &mut ctx.iter_ctx.join_set,
                self.handler.candidate.clone(),
                committee.get_my_pubkey().clone(),
                ctx.round_update.clone(),
                ctx.step,
                ctx.outbound.clone(),
                ctx.inbound.clone(),
                ctx.verified_candidates.clone(),
                self.executor.clone(),
            );
        }

        // handle queued messages for current round and step.
        if let Some(m) =
            ctx.handle_future_msgs(&committee, &mut self.handler).await
        {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler, &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "1st_red"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }
    pub fn get_committee_size(&self) -> usize {
        config::FIRST_REDUCTION_COMMITTEE_SIZE
    }
}
