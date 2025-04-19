// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::{to_str, Block, Header};
use node_data::message::payload::{Validation, Vote};
use node_data::message::{
    AsyncQueue, ConsensusHeader, Message, Payload, SignInfo, SignedStepMessage,
};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn, Instrument};

use crate::commons::{Database, RoundUpdate};
use crate::config::is_emergency_iter;
use crate::errors::{HeaderError, OperationError};
use crate::execution_ctx::ExecutionCtx;
use crate::msg_handler::StepOutcome;
use crate::operations::{Operations, StateRoot, VerificationOutput};
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

        let candidate = candidate.expect("Candidate has already been checked");
        let header = candidate.header();
        let candidate_hash = header.hash;

        let vote = match Self::verify_candidate(
            candidate,
            ru.state_root(),
            executor,
            expected_generator,
        )
        .await
        {
            Ok(_) => Vote::Valid(candidate_hash),
            Err(err) => {
                if !err.must_vote() {
                    warn!(
                        event = "Skipping Validation vote",
                        reason = %err
                    );
                    return;
                }

                error!(
                    event = "Candidate verification failed",
                    reason = %err
                );

                Vote::Invalid(candidate_hash)
            }
        };

        Self::cast_vote(vote, ru, iteration, outbound, inbound).await;
    }

    async fn verify_candidate(
        candidate: &Block,
        state_root: StateRoot,
        executor: Arc<T>,
        expected_generator: PublicKeyBytes,
    ) -> Result<(), OperationError> {
        let header = candidate.header();

        // Verify faults
        executor
            .verify_faults(header.height, candidate.faults())
            .await?;

        // Verify candidate header
        let (_, voters, _) = executor
            .verify_candidate_header(header, &expected_generator)
            .await?;

        // Verify state transition
        let vst_result = executor
            .verify_state_transition(state_root, candidate, &voters)
            .await?;

        // Verify header against state transition output
        Self::check_header_vst(header, &vst_result)?;

        Ok(())
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
        let msg = Message::from(validation);

        // Send vote to peers and to local node
        //
        // In Emergency Mode, only Valid votes are broadcasted
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

    fn check_header_vst(
        header: &Header,
        vst_result: &VerificationOutput,
    ) -> Result<(), HeaderError> {
        // Check the header against `event_bloom` and `state_root` from VST
        if vst_result.event_bloom != header.event_bloom {
            return Err(HeaderError::EventBloomMismatch(
                Box::new(vst_result.event_bloom),
                Box::new(header.event_bloom),
            ));
        }

        if vst_result.state_root != header.state_hash {
            return Err(HeaderError::StateRootMismatch(
                vst_result.state_root,
                header.state_hash,
            ));
        }

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
