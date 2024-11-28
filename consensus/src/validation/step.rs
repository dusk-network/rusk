// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use anyhow::anyhow;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{to_str, Block};
use node_data::message::payload::{Validation, Vote};
use node_data::message::{
    AsyncQueue, ConsensusHeader, Message, Payload, SignInfo, SignedStepMessage,
};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, info, Instrument};

use crate::commons::{Database, RoundUpdate};
use crate::config::is_emergency_iter;
use crate::execution_ctx::ExecutionCtx;
use crate::msg_handler::StepOutcome;
use crate::operations::{Operations, Voter};
use crate::validation::handler;

pub struct ValidationStep<T, D: Database> {
    handler: Arc<Mutex<handler::ValidationHandler<D>>>,
    executor: Arc<T>,
}

impl<T: Operations + 'static, D: Database> ValidationStep<T, D> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_try_vote(
        join_set: &mut JoinSet<()>,
        iteration: u8,
        candidate: Option<Block>,
        ru: RoundUpdate,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
        executor: Arc<T>,
        expected_generator: PublicKeyBytes,
    ) {
        let hash = to_str(
            &candidate
                .as_ref()
                .map(|c| c.header().hash)
                .unwrap_or_default(),
        );
        join_set.spawn(
            async move {
                Self::try_vote(
                    iteration,
                    candidate.as_ref(),
                    &ru,
                    outbound,
                    inbound,
                    executor,
                    expected_generator,
                )
                .await
            }
            .instrument(tracing::info_span!("validation", hash,)),
        );
    }

    pub(crate) async fn try_vote(
        iteration: u8,
        candidate: Option<&Block>,
        ru: &RoundUpdate,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
        executor: Arc<T>,
        expected_generator: PublicKeyBytes,
    ) {
        if candidate.is_none() {
            Self::cast_vote(
                Vote::NoCandidate,
                ru,
                iteration,
                outbound,
                inbound,
            )
            .await;
            return;
        }
        let candidate = candidate.expect("Candidate to be already checked");
        let header = candidate.header();

        // Verify candidate header
        let vote = match executor
            .verify_candidate_header(header, &expected_generator)
            .await
        {
            Ok((_, voters, _)) => {
                // Call Verify State Transition to make sure transactions set is
                // valid

                if let Err(err) = executor
                    .verify_faults(header.height, candidate.faults())
                    .await
                {
                    error!(event = "invalid faults", ?err);
                    Vote::Invalid(header.hash)
                } else {
                    match Self::call_vst(candidate, &voters, &executor).await {
                        Ok(_) => Vote::Valid(header.hash),
                        Err(err) => {
                            error!(event = "failed_vst_call", ?err);
                            Vote::Invalid(header.hash)
                        }
                    }
                }
            }
            Err(err) => {
                let voting = err.must_vote();
                error!(event = "invalid_header", ?err, ?header, voting);
                if !voting {
                    return;
                }
                Vote::Invalid(header.hash)
            }
        };

        Self::cast_vote(vote, ru, iteration, outbound, inbound).await;
    }

    async fn cast_vote(
        vote: Vote,
        ru: &RoundUpdate,
        iteration: u8,
        outbound: AsyncQueue<Message>,
        inbound: AsyncQueue<Message>,
    ) {
        // Sign and construct validation message
        let validation = self::build_validation_payload(vote, ru, iteration);
        let vote = validation.vote;
        let msg = Message::from(validation);

        if vote.is_valid() || !is_emergency_iter(iteration) {
            info!(
              event = "Cast vote",
              step = "Validation",
              info = ?msg.header,
              vote = ?vote
            );

            // Publish
            outbound.try_send(msg.clone());

            // Register my vote locally
            inbound.try_send(msg);
        }
    }

    async fn call_vst(
        candidate: &Block,
        voters: &[Voter],
        executor: &Arc<T>,
    ) -> anyhow::Result<()> {
        match executor.verify_state_transition(candidate, voters).await {
            Ok(output) => {
                // Ensure the `event_bloom` and `state_root` returned
                // from the VST call are the
                // ones we expect to have with the
                // current candidate block.
                if output.event_bloom != candidate.header().event_bloom {
                    return Err(anyhow!(
                        "mismatch, event_bloom: {}, candidate_event_bloom: {}",
                        hex::encode(output.event_bloom),
                        hex::encode(candidate.header().event_bloom)
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

pub fn build_validation_payload(
    vote: Vote,
    ru: &RoundUpdate,
    iteration: u8,
) -> Validation {
    let header = ConsensusHeader {
        prev_block_hash: ru.hash(),
        round: ru.round,
        iteration,
    };

    let sign_info = SignInfo::default();
    let mut validation = Validation {
        header,
        vote,
        sign_info,
    };
    validation.sign(&ru.secret_key, ru.pubkey_bls.inner());
    validation
}

impl<T: Operations + 'static, D: Database> ValidationStep<T, D> {
    pub(crate) fn new(
        executor: Arc<T>,
        handler: Arc<Mutex<handler::ValidationHandler<D>>>,
    ) -> Self {
        Self { handler, executor }
    }

    pub async fn reinitialize(
        &mut self,
        msg: Message,
        round: u64,
        iteration: u8,
    ) {
        let mut handler = self.handler.lock().await;
        handler.reset(iteration);

        if let Payload::Candidate(p) = msg.clone().payload {
            handler.candidate = Some(p.candidate);
        }

        let hash = handler
            .candidate
            .as_ref()
            .map(|c| c.header().hash)
            .unwrap_or_default();

        debug!(
            event = "init",
            name = self.name(),
            round,
            iter = iteration,
            hash = to_str(&hash),
        )
    }

    pub async fn run<DB: Database>(
        &mut self,
        mut ctx: ExecutionCtx<'_, T, DB>,
    ) -> Message {
        let committee = ctx
            .get_current_committee()
            .expect("committee to be created before run");
        if ctx.am_member(committee) {
            let candidate = self.handler.lock().await.candidate.clone();

            // Casting a NIL vote is disabled in Emergency Mode
            let voting_enabled =
                candidate.is_some() || !is_emergency_iter(ctx.iteration);

            let current_generator = ctx
                .iter_ctx
                .get_generator(ctx.iteration)
                .expect("Generator to be created ");
            if voting_enabled {
                Self::spawn_try_vote(
                    &mut ctx.iter_ctx.join_set,
                    ctx.iteration,
                    candidate,
                    ctx.round_update.clone(),
                    ctx.outbound.clone(),
                    ctx.inbound.clone(),
                    self.executor.clone(),
                    current_generator,
                );
            }
        }

        // handle queued messages for current round and step.
        match ctx.handle_future_msgs(self.handler.clone()).await {
            StepOutcome::Ready(m) => m,
            StepOutcome::Pending => {
                ctx.event_loop(self.handler.clone(), None).await
            }
        }
    }

    pub fn name(&self) -> &'static str {
        "validation"
    }
}
