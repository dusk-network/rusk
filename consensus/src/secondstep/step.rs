// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{spawn_send_reduction, ConsensusError, Database};
use crate::config;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use std::marker::PhantomData;

use crate::secondstep::handler;
use crate::user::committee::Committee;
use node_data::ledger::{to_str, Block};
use node_data::message::{Message, Payload, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(unused)]
pub struct Reduction<T, DB> {
    handler: Arc<Mutex<handler::Reduction>>,
    candidate: Option<Block>,
    timeout_millis: u64,
    executor: Arc<Mutex<T>>,

    marker: PhantomData<DB>,
}

impl<T: Operations + 'static, DB: Database> Reduction<T, DB> {
    pub(crate) fn new(
        executor: Arc<Mutex<T>>,
        handler: Arc<Mutex<handler::Reduction>>,
    ) -> Self {
        Self {
            handler,
            candidate: None,
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            executor,
            marker: PhantomData,
        }
    }

    pub async fn reinitialize(&mut self, msg: &Message, round: u64, step: u8) {
        let mut handler = self.handler.lock().await;

        self.candidate = None;
        handler.reset(step);

        if let Payload::StepVotesWithCandidate(p) = msg.payload.clone() {
            handler.first_step_votes = p.sv;
            self.candidate = Some(p.candidate);
        }

        tracing::debug!(
            event = "init",
            name = self.name(),
            round = round,
            step = step,
            timeout = self.timeout_millis,
            hash = to_str(
                &self
                    .candidate
                    .as_ref()
                    .map_or(&Block::default(), |c| c)
                    .header()
                    .hash
            ),
            fsv_bitset = handler.first_step_votes.bitset,
        )
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, DB, T>,
        committee: Committee,
    ) -> Result<Message, ConsensusError> {
        self.handler.lock().await.committees[ctx.step as usize] =
            committee.clone();

        if committee.am_member() {
            //  Send reduction in async way
            if let Some(b) = &self.candidate {
                spawn_send_reduction(
                    &mut ctx.iter_ctx.join_set,
                    ctx.iter_ctx.verified_hash.clone(),
                    b.clone(),
                    committee.get_my_pubkey().clone(),
                    ctx.round_update.clone(),
                    ctx.step,
                    ctx.outbound.clone(),
                    ctx.inbound.clone(),
                    self.executor.clone(),
                    Topics::SecondReduction,
                );
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx
            .handle_future_msgs(&committee, self.handler.clone())
            .await
        {
            return Ok(m);
        }

        ctx.event_loop(
            &committee,
            self.handler.clone(),
            &mut self.timeout_millis,
        )
        .await
    }

    pub fn name(&self) -> &'static str {
        "2nd_red"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }

    pub fn get_committee_size(&self) -> usize {
        config::SECOND_REDUCTION_COMMITTEE_SIZE
    }
}
