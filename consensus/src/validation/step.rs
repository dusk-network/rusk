// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::config;
use crate::execution_ctx::ExecutionCtx;
use crate::operations::Operations;
use crate::validation::handler;
use anyhow::anyhow;
use node_data::ledger::{to_str, Block};
use node_data::message::{self, AsyncQueue, Message, Payload, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, info, Instrument};

pub struct ValidationStep<T> {
    handler: Arc<Mutex<handler::ValidationHandler>>,
    executor: Arc<Mutex<T>>,
}

impl<T: Operations + 'static> ValidationStep<T> {
    pub(crate) fn spawn_try_vote(
        join_set: &mut JoinSet<()>,
        candidate: Block,
        ru: RoundUpdate,
        iteration: u8,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
        executor: Arc<Mutex<T>>,
    ) {
        let hash = to_str(&candidate.header().hash);
        join_set.spawn(
            async move {
                Self::try_vote(
                    &candidate, &ru, iteration, outbound, inbound, executor,
                )
                .await
            }
            .instrument(tracing::info_span!("validation", hash,)),
        );
    }

    pub(crate) async fn try_vote(
        candidate: &Block,
        ru: &RoundUpdate,
        iteration: u8,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
        executor: Arc<Mutex<T>>,
    ) {
        let header = candidate.header();

        // A Validation step with empty/default Block produces a Nil Vote
        if header.hash == [0u8; 32] {
            Self::cast_vote([0u8; 32], ru, iteration, outbound, inbound).await;
            return;
        }

        // Verify candidate header (all fields except the winning certificate)
        // NB: Winning certificate is produced only on reaching consensus
        if let Err(err) = executor
            .lock()
            .await
            .verify_block_header(header, true)
            .await
        {
            error!(event = "invalid_header", ?err, ?header);
            return;
        };

        // Call Verify State Transition to make sure transactions set is valid
        if let Err(err) = Self::call_vst(candidate, executor).await {
            error!(event = "failed_vst_call", ?err);
            return;
        }

        Self::cast_vote(header.hash, ru, iteration, outbound, inbound).await;
    }

    async fn cast_vote(
        hash: [u8; 32],
        ru: &RoundUpdate,
        iteration: u8,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
    ) {
        let hdr = message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            iteration,
            block_hash: hash,
            topic: Topics::Validation,
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        // Sign and construct validation message
        let msg = message::Message::new_validation(
            hdr,
            message::payload::Validation { signature },
        );

        // Publish validation vote
        info!(event = "send_vote");

        // Publish
        outbound.send(msg.clone()).await.unwrap_or_else(|err| {
            error!("could not publish validation {err:?}")
        });

        // Register my vote locally
        inbound.send(msg).await.unwrap_or_else(|err| {
            error!("could not register validation {err:?}")
        });
    }

    async fn call_vst(
        candidate: &Block,
        executor: Arc<Mutex<T>>,
    ) -> anyhow::Result<()> {
        match executor
            .lock()
            .await
            .verify_state_transition(candidate)
            .await
        {
            Ok(output) => {
                // Ensure the `event_hash` and `state_root` returned
                // from the VST call are the
                // ones we expect to have with the
                // current candidate block.
                if output.event_hash != candidate.header().event_hash {
                    return Err(anyhow!(
                        "mismatch, event_hash: {}, candidate_event_hash: {}",
                        hex::encode(output.event_hash),
                        hex::encode(candidate.header().event_hash)
                    ));
                }

                if output.state_root != candidate.header().state_hash {
                    return Err(anyhow!(
                        "mismatch, state_hash: {}, candidate_state_hash: {}",
                        hex::encode(output.state_root),
                        hex::encode(candidate.header().state_hash)
                    ));
                }
            }
            Err(err) => {
                return Err(anyhow!("vm_err: {:?}", err));
            }
        };

        Ok(())
    }
}
impl<T: Operations + 'static> ValidationStep<T> {
    pub(crate) fn new(
        executor: Arc<Mutex<T>>,
        handler: Arc<Mutex<handler::ValidationHandler>>,
    ) -> Self {
        Self { handler, executor }
    }

    pub async fn reinitialize(
        &mut self,
        msg: &Message,
        round: u64,
        iteration: u8,
    ) {
        let mut handler = self.handler.lock().await;
        handler.reset(iteration);

        if let Payload::Candidate(p) = msg.clone().payload {
            handler.candidate = p.candidate.clone();
        }

        debug!(
            event = "init",
            name = self.name(),
            round,
            iter = iteration,
            hash = to_str(&handler.candidate.header().hash),
        )
    }

    pub async fn run<DB: Database>(
        &mut self,
        mut ctx: ExecutionCtx<'_, DB, T>,
    ) -> Result<Message, ConsensusError> {
        let committee = ctx
            .get_current_committee()
            .expect("committee to be created before run");
        if ctx.am_member(committee) {
            let candidate = self.handler.lock().await.candidate.clone();

            // Casting a NIL vote is disabled in Emergency Mode
            let voting_enabled = candidate.header().hash != [0u8; 32]
                || ctx.iteration < config::EMERGENCY_MODE_ITERATION_THRESHOLD;

            if voting_enabled {
                Self::spawn_try_vote(
                    &mut ctx.iter_ctx.join_set,
                    candidate,
                    ctx.round_update.clone(),
                    ctx.iteration,
                    ctx.outbound.clone(),
                    ctx.inbound.clone(),
                    self.executor.clone(),
                );
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(self.handler.clone()).await {
            return Ok(m);
        }

        ctx.event_loop(self.handler.clone()).await
    }

    pub fn name(&self) -> &'static str {
        "validation"
    }
}
