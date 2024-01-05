// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database};
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use node_data::message::Message;
use std::cmp;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config;
use crate::proposal::block_generator::Generator;
use crate::proposal::handler;
use tracing::{debug, error};

pub struct ProposalStep<T, D: Database>
where
    T: Operations,
{
    handler: Arc<Mutex<handler::ProposalHandler<D>>>,
    bg: Generator<T>,
    timeout_millis: u64,
}

impl<T: Operations + 'static, D: Database> ProposalStep<T, D> {
    pub fn new(
        executor: Arc<Mutex<T>>,
        _db: Arc<Mutex<D>>,
        handler: Arc<Mutex<handler::ProposalHandler<D>>>,
    ) -> Self {
        Self {
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            handler,
            bg: Generator::new(executor),
        }
    }

    pub async fn reinitialize(
        &mut self,
        _msg: &Message,
        round: u64,
        iteration: u8,
    ) {
        debug!(
            event = "init",
            name = self.name(),
            round,
            iteration,
            timeout = self.timeout_millis,
        )
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, D, T>,
    ) -> Result<Message, ConsensusError> {
        let committee = ctx
            .get_current_committee()
            .expect("committee to be created before run");
        if ctx.am_member(committee) {
            let iteration =
                cmp::min(config::RELAX_ITERATION_THRESHOLD, ctx.iteration);

            // Fetch failed certificates from sv_registry
            let failed_certificates = ctx
                .sv_registry
                .lock()
                .await
                .get_nil_certificates(0, iteration as usize);

            if let Ok(msg) = self
                .bg
                .generate_candidate_message(
                    &ctx.round_update,
                    ctx.iteration,
                    failed_certificates,
                )
                .await
            {
                // Broadcast the candidate block for this round/iteration.
                if let Err(e) = ctx.outbound.send(msg.clone()).await {
                    error!("could not send newblock msg due to {:?}", e);
                }

                // register new candidate in local state
                match self
                    .handler
                    .lock()
                    .await
                    .collect(msg, &ctx.round_update, ctx.iteration, committee)
                    .await
                {
                    Ok(f) => {
                        if let HandleMsgOutput::Ready(msg) = f {
                            return Ok(msg);
                        }
                    }
                    Err(e) => {
                        error!("invalid candidate generated due to {:?}", e)
                    }
                };
            } else {
                error!("block generator couldn't create candidate block")
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(self.handler.clone()).await {
            return Ok(m);
        }

        ctx.event_loop(self.handler.clone(), &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "sel"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }
    pub fn get_committee_size(&self) -> usize {
        config::PROPOSAL_COMMITTEE_SIZE
    }
}
