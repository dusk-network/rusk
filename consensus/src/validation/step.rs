// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::contract_state::{CallParams, Operations};
use crate::execution_ctx::ExecutionCtx;
use crate::validation::handler;
use anyhow::anyhow;
use dusk_bytes::DeserializableSlice;
use node_data::bls::PublicKey;
use node_data::ledger::{to_str, Block};
use node_data::message::{self, AsyncQueue, Message, Payload, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, Instrument};

pub struct ValidationStep<T> {
    handler: Arc<Mutex<handler::ValidationHandler>>,
    executor: Arc<Mutex<T>>,
}

impl<T: Operations + 'static> ValidationStep<T> {
    #[allow(clippy::too_many_arguments)]
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
            .instrument(tracing::info_span!("voting", hash)),
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
        let hash = candidate.header().hash;

        // Call VST for non-empty blocks
        if hash != [0u8; 32] {
            if let Err(err) = Self::call_vst(candidate, ru, executor).await {
                error!(
                    event = "failed_vst_call",
                    reason = format!("{:?}", err)
                );
                return;
            }
        }

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
        debug!(event = "voting", vtype = "validation", hash = to_str(&hash));

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
        ru: &RoundUpdate,
        executor: Arc<Mutex<T>>,
    ) -> anyhow::Result<()> {
        let pubkey = &candidate.header().generator_bls_pubkey.0;
        let generator = match dusk_bls12_381_sign::PublicKey::from_slice(pubkey)
        {
            Ok(pubkey) => pubkey,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "invalid bls key {}, err: {:?}",
                    hex::encode(pubkey),
                    e,
                ));
            }
        };

        match executor
            .lock()
            .await
            .verify_state_transition(
                CallParams {
                    round: ru.round,
                    block_gas_limit: candidate.header().gas_limit,
                    generator_pubkey: PublicKey::new(generator),
                },
                candidate.txs().clone(),
            )
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
            iteration,
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
