// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp;
use std::sync::Arc;
use std::time::Duration;

use node_data::get_current_timestamp;
use node_data::ledger::IterationsInfo;
use node_data::message::Message;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::commons::Database;
use crate::config;
use crate::config::MINIMUM_BLOCK_TIME;
use crate::execution_ctx::ExecutionCtx;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::operations::Operations;
use crate::proposal::block_generator::Generator;
use crate::proposal::handler;

pub struct ProposalStep<T, D: Database>
where
    T: Operations,
{
    handler: Arc<Mutex<handler::ProposalHandler<D>>>,
    bg: Generator<T>,
}

impl<T: Operations + 'static, D: Database> ProposalStep<T, D> {
    pub fn new(
        executor: Arc<T>,
        _db: Arc<Mutex<D>>,
        handler: Arc<Mutex<handler::ProposalHandler<D>>>,
    ) -> Self {
        Self {
            handler,
            bg: Generator::new(executor),
        }
    }

    pub async fn reinitialize(
        &mut self,
        _msg: Message,
        round: u64,
        iteration: u8,
    ) {
        debug!(event = "init", name = self.name(), round, iter = iteration,)
    }

    pub async fn run(&mut self, mut ctx: ExecutionCtx<'_, T, D>) -> Message {
        let committee = ctx
            .get_current_committee()
            .expect("committee to be created before run");

        let tip_timestamp = ctx.round_update.timestamp();

        if ctx.am_member(committee) {
            let iteration =
                cmp::min(config::RELAX_ITERATION_THRESHOLD, ctx.iteration);

            // Fetch failed attestations from sv_registry
            let failed_attestations =
                ctx.sv_registry.lock().await.get_failed_atts(iteration);

            match self
                .bg
                .generate_candidate_message(
                    &ctx.round_update,
                    ctx.iteration,
                    IterationsInfo::new(failed_attestations),
                )
                .await
            {
                Ok(msg) => {
                    debug!(
                        event = "send message",
                        src = "proposal",
                        msg_topic = ?msg.topic(),
                        info = ?msg.header,
                        ray_id = msg.ray_id()
                    );
                    ctx.outbound.try_send(msg.clone());

                    // register new candidate in local state
                    match self
                        .handler
                        .lock()
                        .await
                        .collect(
                            msg,
                            &ctx.round_update,
                            committee,
                            None,
                            &ctx.iter_ctx.committees,
                        )
                        .await
                    {
                        Ok(StepOutcome::Ready(msg)) => {
                            Self::wait_until_next_slot(tip_timestamp).await;
                            return msg;
                        }
                        Err(e) => {
                            error!("invalid candidate generated due to {:?}", e)
                        }
                        _ => {}
                    };
                }

                Err(e) => {
                    error!(
                      event = "Failed to generate candidate block",
                      round = ctx.round_update.round,
                      iteration = ctx.iteration,
                      err = ?e,
                    )
                }
            }
        }

        let additional_timeout = Self::next_slot_in(tip_timestamp);
        let msg = match ctx.handle_future_msgs(self.handler.clone()).await {
            StepOutcome::Ready(m) => m,
            StepOutcome::Pending => {
                ctx.event_loop(self.handler.clone(), additional_timeout)
                    .await
            }
        };
        Self::wait_until_next_slot(tip_timestamp).await;
        msg
    }

    /// Waits until the next slot is reached
    async fn wait_until_next_slot(tip_timestamp: u64) {
        if let Some(delay) = Self::next_slot_in(tip_timestamp) {
            info!(event = "next_slot", ?delay);
            tokio::time::sleep(delay).await;
        }
    }

    /// Calculate the duration needed to the next slot
    fn next_slot_in(tip_timestamp: u64) -> Option<Duration> {
        let current_time_secs = get_current_timestamp();

        let next_slot_timestamp = tip_timestamp + *MINIMUM_BLOCK_TIME;
        if current_time_secs >= next_slot_timestamp {
            None
        } else {
            // block_timestamp - localtime
            Some(Duration::from_secs(next_slot_timestamp - current_time_secs))
        }
    }

    pub fn name(&self) -> &'static str {
        "proposal"
    }
}
