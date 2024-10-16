// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{Database, QuorumMsgSender, RoundUpdate};

use crate::errors::ConsensusError;
use crate::iteration_ctx::IterationCtx;
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::operations::Operations;
use crate::queue::{MsgRegistry, MsgRegistryError};
use crate::ratification::handler::RatificationHandler;
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::Block;
use node_data::message::payload::{
    QuorumType, RatificationResult, ValidationResult, Vote,
};
use node_data::message::{AsyncQueue, Message, Payload};
use node_data::StepName;

use crate::config::{is_emergency_iter, CONSENSUS_MAX_ITER};
use crate::ratification::step::RatificationStep;
use crate::validation::step::ValidationStep;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

/// ExecutionCtx encapsulates all data needed in the execution of consensus
/// messages handlers.
pub struct ExecutionCtx<'a, T, DB: Database> {
    pub iter_ctx: &'a mut IterationCtx<DB>,

    /// Messaging-related fields
    pub inbound: AsyncQueue<Message>,
    pub outbound: AsyncQueue<Message>,
    pub future_msgs: Arc<Mutex<MsgRegistry<Message>>>,

    /// State-related fields
    pub provisioners: &'a Provisioners,

    // Round/Step parameters
    pub round_update: RoundUpdate,
    pub iteration: u8,
    step: StepName,
    step_start_time: Option<Instant>,

    pub client: Arc<T>,

    pub sv_registry: SafeAttestationInfoRegistry,
    quorum_sender: QuorumMsgSender,
}

impl<'a, T: Operations + 'static, DB: Database> ExecutionCtx<'a, T, DB> {
    /// Creates step execution context.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        iter_ctx: &'a mut IterationCtx<DB>,
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        future_msgs: Arc<Mutex<MsgRegistry<Message>>>,
        provisioners: &'a Provisioners,
        round_update: RoundUpdate,
        iteration: u8,
        step: StepName,
        client: Arc<T>,
        sv_registry: SafeAttestationInfoRegistry,
        quorum_sender: QuorumMsgSender,
    ) -> Self {
        Self {
            iter_ctx,
            inbound,
            outbound,
            future_msgs,
            provisioners,
            round_update,
            iteration,
            step,
            client,
            sv_registry,
            quorum_sender,
            step_start_time: None,
        }
    }

    pub fn step_name(&self) -> StepName {
        self.step
    }

    pub fn step(&self) -> u8 {
        self.step.to_step(self.iteration)
    }

    pub fn set_start_time(&mut self) {
        self.step_start_time = Some(Instant::now());
    }

    /// Returns true if `my pubkey` is a member of [`committee`].
    pub(crate) fn am_member(&self, committee: &Committee) -> bool {
        committee.is_member(&self.round_update.pubkey_bls)
    }

    pub(crate) fn get_current_committee(&self) -> Option<&Committee> {
        self.iter_ctx.committees.get_committee(self.step())
    }

    /// Returns true if the last step of last iteration is currently running
    fn last_step_running(&self) -> bool {
        self.iteration == CONSENSUS_MAX_ITER - 1
            && self.step_name() == StepName::Ratification
    }

    /// Runs a loop that collects both inbound messages and timeout event.
    ///
    /// It accepts an instance of MsgHandler impl (phase var) and calls its
    /// methods based on the occurred event.
    ///
    /// In an event of timeout, it also increases the step timeout value
    /// accordingly.
    ///
    /// By design, the loop is terminated by aborting the consensus task.
    pub async fn event_loop<C: MsgHandler>(
        &mut self,
        phase: Arc<Mutex<C>>,
    ) -> Result<Message, ConsensusError> {
        let open_consensus_mode = self.last_step_running();

        // When consensus is in open_consensus_mode then it keeps Ratification
        // step running indefinitely until either a valid block or
        // emergency block is accepted
        let timeout = if open_consensus_mode {
            let dur = Duration::new(u32::MAX as u64, 0);
            info!(event = "run event_loop", ?dur, mode = "open_consensus",);
            dur
        } else {
            let dur = self.iter_ctx.get_timeout(self.step_name());
            debug!(event = "run event_loop", ?dur);
            dur
        };

        let deadline = Instant::now().checked_add(timeout).unwrap();
        let inbound = self.inbound.clone();

        // Handle both timeout event and messages from inbound queue.
        loop {
            match time::timeout_at(deadline, inbound.recv()).await {
                // Inbound message event
                Ok(Ok(msg)) => {
                    match msg.payload {
                        Payload::Candidate(_)
                        | Payload::Validation(_)
                        | Payload::Ratification(_) => {
                            // If we received a Step Message, we pass it on to
                            // the running step for processing.
                            if let Some(step_result) = self
                                .process_inbound_msg(phase.clone(), msg.clone())
                                .await
                            {
                                info!(
                                    event = "step completed",
                                    result = ?step_result.payload
                                );

                                // In the normal case, we just return the result
                                // to Consensus
                                if !open_consensus_mode {
                                    self.report_elapsed_time().await;
                                    return Ok(step_result);
                                }

                                // In Open Consensus, we only broadcast Success
                                // Quorums
                                if let Payload::Quorum(qmsg) =
                                    &step_result.payload
                                {
                                    let qround = qmsg.header.round;
                                    let qiter = qmsg.header.iteration;

                                    match qmsg.att.result {
                                        RatificationResult::Success(vote) => {
                                            info!(
                                                event = "New Quorum",
                                                round = qround,
                                                iter = qiter,
                                                vote = ?vote,
                                                is_local = true
                                            );

                                            self.quorum_sender
                                                .send_quorum(msg)
                                                .await;
                                        }
                                        RatificationResult::Fail(vote) => {
                                            debug!(
                                              event = "Quorum discarded",
                                              reason = "Fail Quorum in Open Consensus mode",
                                              round = qround,
                                              iter = qiter,
                                              vote = ?vote,
                                              is_local = true
                                            );
                                        }
                                    }
                                }

                                // In open consensus mode, the step is only
                                // terminated when accepting a block
                                continue;
                            }
                        }

                        // Handle Quorum messages from the network
                        Payload::Quorum(ref qmsg) => {
                            let round = self.round_update.round;
                            let prev = self.round_update.hash();
                            let iter = self.iteration;

                            let qround = qmsg.header.round;
                            let qiter = qmsg.header.iteration;
                            let qprev = qmsg.header.prev_block_hash;

                            // We only handle messages for the current round and
                            // branch, and iteration <= current_iteration
                            if qround != round || qprev != prev || qiter > iter
                            {
                                debug!(
                                  event = "Quorum discarded",
                                  reason = "past round/iter, or fork",
                                  round = qround,
                                  iter = qiter,
                                  vote = ?qmsg.vote(),
                                  is_local = false
                                );
                                continue;
                            }

                            let att = qmsg.att;

                            // Handle Open Consensus mode
                            if open_consensus_mode {
                                match att.result {
                                    RatificationResult::Success(vote) => {
                                        info!(
                                            event = "New Quorum",
                                            round = qround,
                                            iter = qiter,
                                            vote = ?vote,
                                            is_local = false
                                        );

                                        // Repropagate Success Quorum
                                        self.quorum_sender
                                            .send_quorum(msg.clone())
                                            .await;
                                    }
                                    RatificationResult::Fail(vote) => {
                                        debug!(
                                          event = "Quorum discarded",
                                          reason = "Fail Quorum in Open Consensus",
                                          round = qround,
                                          iter = qiter,
                                          vote = ?vote,
                                          is_local = false
                                        );
                                    }
                                }

                                // In Open Consensus, we only stop upon
                                // accepting a block
                                continue;
                            }

                            // Store Fail Attestations in the Registry.
                            //
                            // INFO: We do it here so we can store
                            // past-iteration Attestations without interrupting
                            // the step execution
                            if let RatificationResult::Fail(vote) = att.result {
                                match vote {
                                    Vote::NoCandidate | Vote::Invalid(_) => {
                                        // Check if we know this Attestation
                                        let mut sv_registry =
                                            self.sv_registry.lock().await;

                                        match sv_registry.get_fail_att(qiter) {
                                            None => {
                                                debug!(
                                                  event = "Storing Fail Attestation",
                                                  round = qround,
                                                  iter = qiter,
                                                  vote = ?vote,
                                                );

                                                let generator = self
                                                    .iter_ctx
                                                    .get_generator(qiter);

                                                sv_registry
                                                        .set_attestation(
                                                          qiter,
                                                          att,
                                                          &generator.expect("There must be a valid generator")
                                                        );
                                            }
                                            Some(_) => {
                                                debug!(
                                                  event = "Quorum discarded",
                                                  reason = "known Fail Quorum",
                                                  round = qround,
                                                  iter = qiter,
                                                  vote = ?vote,
                                                );
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            // If we receive a Quorum message for the
                            // current iteration, we terminate the step and
                            // pass the message to the Consensus task to
                            // terminate the iteration.
                            // Messages for different iterations are discarded.
                            if qiter == iter {
                                return Ok(msg);
                            } else {
                                debug!(
                                  event = "Quorum discarded",
                                  reason = "different iteration",
                                  round = qround,
                                  iter = qiter,
                                  vote = ?qmsg.vote(),
                                );
                            }
                        }
                        _ => {
                            warn!("Unexpected msg received in Consensus")
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Error while receiving msg: {e}");
                }
                // Timeout event. Phase could not reach its final goal.
                // Increase timeout for next execution of this step and move on.
                Err(_) => {
                    info!(event = "timeout-ed");
                    if open_consensus_mode {
                        error!("Timeout detected during last step running. This should never happen")
                    } else {
                        self.process_timeout_event(phase).await;
                        return Ok(Message::empty());
                    }
                }
            }
        }
    }

    /// Cast a validation vote for a candidate that originates from former
    /// iteration
    pub(crate) async fn try_cast_validation_vote(&mut self, candidate: &Block) {
        let msg_iteration = candidate.header().iteration;
        let step = StepName::Validation.to_step(msg_iteration);

        if let Some(committee) = self.iter_ctx.committees.get_committee(step) {
            if self.am_member(committee) {
                let expected_generator = self
                    .iter_ctx
                    .get_generator(msg_iteration)
                    .expect("generator to exists");
                ValidationStep::try_vote(
                    msg_iteration,
                    Some(candidate),
                    &self.round_update,
                    self.outbound.clone(),
                    self.inbound.clone(),
                    self.client.clone(),
                    expected_generator,
                )
                .await;
            };
        } else {
            error!(event = "committee not found", msg_iteration);
        }
    }

    pub(crate) async fn try_cast_ratification_vote(
        &self,
        msg_iteration: u8,
        validation: &ValidationResult,
    ) {
        let step = StepName::Ratification.to_step(msg_iteration);

        if let Some(committee) = self.iter_ctx.committees.get_committee(step) {
            if self.am_member(committee) {
                // Should we collect our own vote?
                let _msg = RatificationStep::try_vote(
                    &self.round_update,
                    msg_iteration,
                    validation,
                    self.outbound.clone(),
                )
                .await;
            }
        }
    }

    /// Process messages from past
    ///
    /// Ignores messages that do not originate from emergency iteration of
    /// current round
    async fn handle_past_msg(&mut self, msg: Message) {
        if msg.header.round != self.round_update.round {
            log_msg("discarded message", "handle_past_msg", &msg);
            // should we send current tip to the msg sender?
            return;
        }
        // Repropagate past iteration messages (they have been already
        // validated)

        log_msg("outbound send", "handle_past_msg", &msg);
        self.outbound.try_send(msg.clone());

        if is_emergency_iter(msg.header.iteration) {
            self.on_emergency_mode(msg).await;
        }
    }

    /// Handles a consensus message in emergency mode
    async fn on_emergency_mode(&mut self, msg: Message) {
        let mut msg = msg;
        // Try to vote for a candidate block from former iteration
        if let Payload::Candidate(p) = &msg.payload {
            self.try_cast_validation_vote(&p.candidate).await;
        }
        let msg_iteration = msg.header.iteration;

        // Try to vote for a validation result from former iteration
        // This is not implemented yet, no one is requesting it
        if let Payload::ValidationQuorum(p) = &msg.payload {
            if !p.result.vote().is_valid() {
                return;
            }
            if let Err(e) = RatificationHandler::verify_validation_result(
                &p.header,
                &p.result,
                &self.iter_ctx.committees,
            ) {
                warn!(
                    event = "invalid validation result",
                    src = "emergency_iter",
                    msg_iteration,
                    ?e
                );
                return;
            }
            info!(
                event = "Validation Result from ValidationQuorum",
                src = "emergency_iter",
                msg_iteration,
            );
            msg = p.result.clone().into();
        }

        // Rebroadcast emergency message if it's not invalid
        self.outbound.try_send(msg.clone());

        // Collect message from a previous iteration/step.
        if let Some(m) = self
            .iter_ctx
            .process_past_msg(&self.round_update, msg)
            .await
        {
            match &m.payload {
                Payload::Quorum(q) => {
                    // When collecting votes from a past iteration, only
                    // quorum for Vote::Valid should be propagated
                    if let Vote::Valid(_) = &q.vote() {
                        info!(
                            event = "Quorum",
                            src = "emergency_iter",
                            msg_iteration,
                            vote = ?q.vote(),
                        );

                        self.quorum_sender.send_quorum(m).await;
                    }
                }

                Payload::ValidationResult(result) => {
                    info!(
                      event = "Validation result",
                      src = "emergency_iter",
                      msg_iteration,
                      vote = ?result.vote(),
                      quorum = ?result.quorum(),
                    );

                    if let QuorumType::Valid = result.quorum() {
                        self.try_cast_ratification_vote(msg_iteration, result)
                            .await
                    }
                }
                _ => {
                    // Not supported.
                }
            }
        }
    }

    /// Delegates the received message to the Phase handler for further
    /// processing.
    ///
    /// Returning Option::Some here is interpreted as FinalMessage for the
    /// current iteration by event_loop.
    ///
    /// If the message belongs to a former iteration, it returns None (even if
    /// the message is processed due to emergency mode)
    async fn process_inbound_msg<C: MsgHandler>(
        &mut self,
        phase: Arc<Mutex<C>>,
        msg: Message,
    ) -> Option<Message> {
        // If it's a message from a future iteration of the current round, we
        // generate the committees so that we can pre-verify its validity.
        // We do it here because we need the IterationCtx

        let same_prev_hash = msg.header.round == self.round_update.round
            && msg.header.prev_block_hash == self.round_update.hash();

        if same_prev_hash && msg.header.iteration > self.iteration {
            // Generate committees for the iteration
            self.iter_ctx.generate_iteration_committees(
                msg.header.iteration,
                self.provisioners,
                self.round_update.seed(),
            );
        }

        let committee = self
            .get_current_committee()
            .expect("committee to be created before run");

        let generator = self.get_curr_generator();

        // Check if message is valid in the context of current step
        let valid = phase.lock().await.is_valid(
            &msg,
            &self.round_update,
            self.iteration,
            self.step,
            committee,
            &self.iter_ctx.committees,
        );

        match valid {
            Ok(_) => {
                log_msg("outbound send", "inbound message", &msg);
                // Re-publish the returned message
                self.outbound.try_send(msg.clone());
            }
            // This is a message from future round or step.
            // Save it in future_msgs to be processed when we reach
            // same round/step.
            Err(ConsensusError::FutureEvent) => {
                const SRC: &str = "inbound future message";
                if !same_prev_hash {
                    if let Some(signer) = msg.get_signer() {
                        if !self
                            .provisioners
                            .eligibles(msg.header.round)
                            .any(|(p, _)| p == &signer)
                        {
                            log_msg("discarded msg (not eligible)", SRC, &msg);
                            return None;
                        }
                    }
                }

                if msg.header.round > self.round_update.round + 10 {
                    log_msg("discarded msg (round too far from now)", SRC, &msg);
                    return None;
                }

                // TODO: add additional Error to discard future messages too far
                match self.future_msgs.lock().await.put_msg(msg) {
                    Ok(msg) => {
                        log_msg("outbound send", SRC, &msg);
                        self.outbound.try_send(msg);
                    }
                    Err(MsgRegistryError::NoSigner(msg)) => {
                        log_msg("discarded msg (no signer)", SRC, &msg);
                    }
                    Err(MsgRegistryError::SignerAlreadyEnqueue(msg)) => {
                        log_msg("discarded msg (duplicated)", SRC, &msg);
                    }
                }

                return None;
            }
            Err(ConsensusError::PastEvent) => {
                self.handle_past_msg(msg).await;
                return None;
            }
            Err(ConsensusError::InvalidValidation(QuorumType::NoQuorum)) => {
                warn!(event = "No quorum reached", iter = msg.header.iteration);
                return None;
            }
            // An error here means this message is invalid due to failed
            // verification.
            Err(e) => {
                error!("phase handler err: {:?}", e);
                return None;
            }
        }

        let msg_topic = msg.topic();
        let msg_iter = msg.header.iteration;
        let msg_step = msg.get_step();
        let msg_height = msg.header.round;
        trace!("collecting msg {msg:#?}");

        let collected = phase
            .lock()
            .await
            .collect(msg, &self.round_update, committee, generator)
            .await;

        match collected {
            // Fully valid state reached on this step. Return it as an output to
            // populate next step with it.
            Ok(HandleMsgOutput::Ready(m)) => Some(m),
            // Message collected but phase didn't reach a final result
            Ok(HandleMsgOutput::Pending) => None,
            Err(err) => {
                let event = "failed collect";
                error!(event, ?err, ?msg_topic, msg_iter, msg_step, msg_height,);
                None
            }
        }
    }

    /// Delegates the received event of timeout to the Phase handler for further
    /// processing.
    async fn process_timeout_event<C: MsgHandler>(
        &mut self,
        phase: Arc<Mutex<C>>,
    ) {
        self.iter_ctx.on_timeout_event(self.step_name());

        if let Some(msg) = phase
            .lock()
            .await
            .handle_timeout(&self.round_update, self.iteration)
        {
            log_msg("outbound send", "process timeout event", &msg);
            self.outbound.try_send(msg.clone());
        }
    }

    /// Handles all messages stored in future_msgs queue that belongs to the
    /// current round and step.
    ///
    /// Returns Some(msg) if the step is finalized.
    pub async fn handle_future_msgs<C: MsgHandler>(
        &self,
        phase: Arc<Mutex<C>>,
    ) -> Option<Message> {
        let committee = self
            .get_current_committee()
            .expect("committee to be created before run");

        let generator = self.get_curr_generator();

        if let Some(messages) = self
            .future_msgs
            .lock()
            .await
            .drain_msg_by_round_step(self.round_update.round, self.step())
        {
            if !messages.is_empty() {
                debug!(event = "drain future msgs", count = messages.len(),)
            }

            for msg in messages {
                let ret = phase.lock().await.is_valid(
                    &msg,
                    &self.round_update,
                    self.iteration,
                    self.step,
                    committee,
                    &self.iter_ctx.committees,
                );
                if ret.is_ok() {
                    // Re-publish a drained message
                    log_msg("outbound send", "future_msgs", &msg);

                    self.outbound.try_send(msg.clone());

                    if let Ok(HandleMsgOutput::Ready(msg)) = phase
                        .lock()
                        .await
                        .collect(msg, &self.round_update, committee, generator)
                        .await
                    {
                        return Some(msg);
                    }
                }
            }
        }

        None
    }

    /// Reports step elapsed time to the client
    async fn report_elapsed_time(&mut self) {
        let elapsed = self
            .step_start_time
            .take()
            .expect("valid start time")
            .elapsed();

        let _ = self
            .client
            .add_step_elapsed_time(
                self.round_update.round,
                self.step_name(),
                elapsed,
            )
            .await;
    }

    pub(crate) fn get_curr_generator(&self) -> Option<PublicKeyBytes> {
        self.iter_ctx.get_generator(self.iteration)
    }
}

#[inline(always)]
fn log_msg(event: &str, src: &str, msg: &Message) {
    debug!(
        event,
        src,
        msg_step = msg.get_step(),
        msg_iter = msg.get_iteration(),
        msg_height = msg.get_height(),
        msg_topic = ?msg.topic(),
        ray_id = msg.ray_id()
    );
}
