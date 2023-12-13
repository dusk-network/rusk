// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{spawn_cast_vote, ConsensusError, Database};
use crate::config;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use crate::validation::handler;

use node_data::ledger::to_str;
use node_data::message::{Message, Payload, Topics};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

pub struct ValidationStep<T, DB: Database> {
    timeout_millis: u64,
    handler: Arc<Mutex<handler::ValidationHandler<DB>>>,
    executor: Arc<Mutex<T>>,
}
impl<T: Operations + 'static, DB: Database> ValidationStep<T, DB> {
    pub(crate) fn new(
        executor: Arc<Mutex<T>>,
        _db: Arc<Mutex<DB>>,
        handler: Arc<Mutex<handler::ValidationHandler<DB>>>,
    ) -> Self {
        Self {
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            handler,
            executor,
        }
    }

    pub async fn reinitialize(&mut self, msg: &Message, round: u64, step: u8) {
        let mut handler = self.handler.lock().await;

        handler.reset(step);

        if let Payload::Candidate(p) = msg.clone().payload {
            handler.candidate = p.deref().candidate.clone();
        }

        debug!(
            event = "init",
            name = self.name(),
            round = round,
            step = step,
            timeout = self.timeout_millis,
            hash = to_str(&handler.candidate.header().hash),
        )
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, DB, T>,
    ) -> Result<Message, ConsensusError> {
        let committee = ctx
            .get_committee()
            .expect("committee to be created before run");
        if committee.am_member() {
            let candidate = self.handler.lock().await.candidate.clone();
            // Send reduction async
            spawn_cast_vote(
                &mut ctx.iter_ctx.join_set,
                ctx.iter_ctx.verified_hash.clone(),
                candidate,
                ctx.round_update.pubkey_bls.clone(),
                ctx.round_update.clone(),
                ctx.step,
                ctx.outbound.clone(),
                ctx.inbound.clone(),
                self.executor.clone(),
                Topics::Validation,
            );
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(self.handler.clone()).await {
            return Ok(m);
        }

        ctx.event_loop(self.handler.clone(), &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "1st_red"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }
    pub fn get_committee_size(&self) -> usize {
        config::VALIDATION_COMMITTEE_SIZE
    }
}
