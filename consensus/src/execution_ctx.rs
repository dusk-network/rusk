// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;
use std::time::Duration;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::Block;
use node_data::message::payload::{
    QuorumType, RatificationResult, ValidationResult, Vote,
};
use node_data::message::{AsyncQueue, Message, Payload, Topics};
use node_data::StepName;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use crate::commons::{Database, RoundUpdate};
use crate::config::{
    is_emergency_iter, CONSENSUS_MAX_ITER, MAX_ROUND_DISTANCE,
};
use crate::errors::ConsensusError;
use crate::iteration_ctx::IterationCtx;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::operations::Operations;
use crate::queue::{MsgRegistry, MsgRegistryError};
use crate::ratification::step::RatificationStep;
use crate::step_votes_reg::SafeAttestationInfoRegistry;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::validation::step::ValidationStep;

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

    pub att_registry: SafeAttestationInfoRegistry,
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
        att_registry: SafeAttestationInfoRegistry,
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
            att_registry,
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
    fn is_last_step(&self) -> bool {
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
        additional_timeout: Option<Duration>,
    ) -> Message {
        let round = self.round_update.round;
        let iter = self.iteration;
        let step = self.step_name();

        let mut open_consensus_mode = false;

        let step_timeout = self.iter_ctx.get_timeout(step);
        let timeout = step_timeout + additional_timeout.unwrap_or_default();

        debug!(
            event = "Start step loop",
            ?step,
            round,
            iter,
            ?step_timeout,
            ?additional_timeout
        );

        let mut deadline = Instant::now().checked_add(timeout).unwrap();
        let inbound = self.inbound.clone();

        // Handle both timeout event and messages from inbound queue.
        loop {
            match time::timeout_at(deadline, inbound.recv()).await {
                // Inbound message event
                Ok(Ok(msg)) => {
                    match msg.payload {
                        Payload::Candidate(_)
                        | Payload::Validation(_)
                        | Payload::Ratification(_)
                        | Payload::ValidationQuorum(_) => {
                            // If we received a Step Message, we pass it on to
                            // the running step for processing.
                            if let Some(step_result) = self
                                .process_inbound_msg(phase.clone(), msg.clone())
                                .await
                            {
                                // In the normal case, we just return the result
                                // to Consensus
                                if !open_consensus_mode {
                                    self.report_elapsed_time().await;
                                    return step_result;
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

                                            // Broadcast Quorum
                                            self.outbound.try_send(msg);
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
                                  reason = "past round/future iteration/fork",
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

                                        // Broadcast Success Quorum
                                        self.outbound.try_send(msg.clone());
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

                            match att.result {
                                RatificationResult::Fail(vote) => {
                                    // Store Fail Attestations in the Registry.
                                    //
                                    // INFO: We do it here so we can store
                                    // past-iteration Attestations without
                                    // interrupting the step execution

                                    let mut att_registry =
                                        self.att_registry.lock().await;

                                    match att_registry.get_fail_att(qiter) {
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

                                            att_registry
                                            .set_attestation(
                                              qiter,
                                              att,
                                              &generator.expect("There must be a valid generator")
                                            );
                                        }

                                        Some(_) => {
                                            debug!(
                                              event = "Quorum discarded",
                                              reason = "known Fail Attestation",
                                              round = qround,
                                              iter = qiter,
                                              vote = ?vote,
                                            );
                                        }
                                    }

                                    if qiter != iter {
                                        continue;
                                    }
                                }

                                RatificationResult::Success(_) => {
                                    if qiter != iter {
                                        debug!(
                                          event = "Quorum discarded",
                                          reason = "past iteration",
                                          round = qround,
                                          iter = qiter,
                                          vote = ?qmsg.vote(),
                                        );

                                        continue;
                                    }
                                }
                            }

                            // If we received a Quorum message for the
                            // current iteration, we terminate the step and
                            // pass the message to the Consensus task to
                            // terminate the iteration.
                            return msg;
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
                    info!(event = "Step timeout expired", ?step, round, iter);

                    if self.is_last_step() {
                        info!(event = "Step ended", ?step, round, iter);

                        // If the last step expires, we enter Open Consensus
                        // mode. In this mode, the last step (Ratification)
                        // keeps running indefinitely, until a block is
                        // accepted.
                        info!(event = "Entering Open Consensus mode", round);

                        let timeout = Duration::new(u32::MAX as u64, 0);
                        deadline = Instant::now().checked_add(timeout).unwrap();

                        open_consensus_mode = true;
                    } else {
                        self.process_timeout_event(phase).await;
                        return Message::empty();
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
                debug!(
                    event = "Cast past vote",
                    step = "Validation",
                    mode = "emergency",
                    round = candidate.header().height,
                    iter = msg_iteration
                );

                let expected_generator = self
                    .iter_ctx
                    .get_generator(msg_iteration)
                    .expect("generator to exists");
                ValidationStep::<_, DB>::try_vote(
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
                debug!(
                    event = "Cast past vote",
                    step = "Ratification",
                    mode = "emergency",
                    round = self.round_update.round,
                    iter = msg_iteration
                );

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
        // Discard past-round messages
        if msg.header.round != self.round_update.round {
            log_msg("discarded message (past round)", "handle_past_msg", &msg);
            // should we send current tip to the msg sender?
            return;
        }

        let msg_topic = msg.topic();
        // Repropagate past iteration messages
        // INFO: messages are previously validate by is_valid
        if msg_topic != Topics::ValidationQuorum {
            log_msg("send message", "handle_past_msg", &msg);
            self.outbound.try_send(msg.clone());
        }

        let msg_iteration = msg.header.iteration;

        // Past-iteration messages are only handled in emergency mode
        if !is_emergency_iter(msg_iteration) {
            log_msg(
                "discarded message (past iter in normal mode)",
                "handle_past_msg",
                &msg,
            );
            // TODO: send Inv(Tip) to peer
            return;
        }

        // Process message from a previous iteration/step.
        if let Some(m) = self.iter_ctx.process_past_msg(msg).await {
            match &m.payload {
                Payload::Candidate(p) => {
                    self.try_cast_validation_vote(&p.candidate).await;
                }

                Payload::ValidationResult(result) => {
                    info!(
                      event = "New ValidationResult",
                      mode = "emergency",
                      info = ?m.header,
                      vote = ?result.vote(),
                      src = ?msg_topic
                    );

                    if let QuorumType::Valid = result.quorum() {
                        self.try_cast_ratification_vote(msg_iteration, result)
                            .await
                    }
                }

                Payload::Quorum(q) => {
                    // When collecting votes from a past iteration, only
                    // quorum for Vote::Valid should be propagated
                    if let Vote::Valid(_) = &q.vote() {
                        info!(
                            event = "New Quorum",
                            mode = "emergency",
                            round = q.header.round,
                            iter = q.header.iteration,
                            vote = ?q.vote(),
                        );

                        // Broadcast Quorum
                        self.outbound.try_send(m);
                    }
                }

                _ => {
                    // Validation and Ratification messages should never be
                    // returned by process_past_msg
                    warn!("Invalid message returned by process_past_msg. This should be a bug.")
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

        let current_round = self.round_update.round;

        let same_prev_hash = msg.header.round == current_round
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
                log_msg("send message", "inbound message", &msg);
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

                // We verify message signatures only for the next 10 round
                // messages. Removing this check will lead to
                // repropagate everything only according to the signer pk
                if msg.header.round > current_round + MAX_ROUND_DISTANCE {
                    log_msg(
                        "discarded msg (round too far from now)",
                        SRC,
                        &msg,
                    );
                    return None;
                }

                // TODO: add additional Error to discard future messages too far
                match self.future_msgs.lock().await.put_msg(msg) {
                    Ok(msg) => {
                        log_msg("send message", SRC, &msg);
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
            .collect(
                msg,
                &self.round_update,
                committee,
                generator,
                &self.iter_ctx.committees,
            )
            .await;

        match collected {
            // Fully valid state reached on this step. Return it as an output to
            // populate next step with it.
            Ok(StepOutcome::Ready(m)) => Some(m),
            // Message collected but phase didn't reach a final result
            Ok(StepOutcome::Pending) => None,
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
            log_msg("send message", "process timeout event", &msg);
            self.outbound.try_send(msg.clone());
        }
    }

    /// Handles all messages stored in future_msgs queue that belongs to the
    /// current round and step.
    ///
    /// Returns the step result if the step is finalized.
    pub async fn handle_future_msgs<C: MsgHandler>(
        &self,
        phase: Arc<Mutex<C>>,
    ) -> StepOutcome {
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
                    log_msg("send message", "future_msgs", &msg);

                    self.outbound.try_send(msg.clone());

                    match phase
                        .lock()
                        .await
                        .collect(
                            msg,
                            &self.round_update,
                            committee,
                            generator,
                            &self.iter_ctx.committees,
                        )
                        .await
                    {
                        Ok(StepOutcome::Ready(msg)) => {
                            return StepOutcome::Ready(msg)
                        }
                        Ok(_) => {}
                        Err(e) => warn!("error in collecting message {e:?}"),
                    }
                }
            }
        }

        StepOutcome::Pending
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use node_data::ledger::{Attestation, Block, Header, StepVotes};
    use node_data::message::payload::{Candidate, ValidationQuorum};
    use node_data::message::{ConsensusHeader, SignedStepMessage};
    use dusk_core::signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use tokio::sync::Mutex;

    use crate::commons::TimeoutSet;
    use crate::config::{MIN_STEP_TIMEOUT, TIMEOUT_INCREASE};
    use crate::iteration_ctx::IterationCtx;
    use crate::merkle::merkle_root;
    use crate::operations::{StateTransitionData, StateTransitionResult, Voter};
    use crate::proposal::handler::ProposalHandler;
    use crate::ratification::handler::RatificationHandler;
    use crate::step_votes_reg::AttInfoRegistry;
    use crate::validation::handler::ValidationHandler;
    use crate::user::provisioners::DUSK;

    #[derive(Default)]
    struct DummyDb {
        stored_candidates: Arc<Mutex<usize>>,
        validation_results:
            Arc<Mutex<Vec<(node_data::message::ConsensusHeader, ValidationResult)>>>,
    }

    #[async_trait]
    impl Database for DummyDb {
        async fn store_candidate_block(&mut self, _b: Block) {
            let mut count = self.stored_candidates.lock().await;
            *count += 1;
        }
        async fn store_validation_result(
            &mut self,
            ch: &node_data::message::ConsensusHeader,
            vr: &node_data::message::payload::ValidationResult,
        ) {
            let mut results = self.validation_results.lock().await;
            results.push((*ch, vr.clone()));
        }
        async fn get_last_iter(&self) -> (node_data::ledger::Hash, u8) {
            ([0u8; 32], 0)
        }
        async fn store_last_iter(
            &mut self,
            _data: (node_data::ledger::Hash, u8),
        ) {
        }
    }

    #[derive(Default)]
    struct DummyOps;

    #[async_trait]
    impl Operations for DummyOps {
        async fn validate_block_header(
            &self,
            _candidate_header: &node_data::ledger::Header,
            _expected_generator: &node_data::bls::PublicKeyBytes,
        ) -> Result<Vec<Voter>, crate::errors::HeaderError> {
            Ok(vec![])
        }
        async fn validate_faults(
            &self,
            _block_height: u64,
            _faults: &[node_data::ledger::Fault],
        ) -> Result<(), crate::errors::OperationError> {
            Ok(())
        }
        async fn validate_state_transition(
            &self,
            _prev_state: [u8; 32],
            _blk: &Block,
            _cert_voters: &[Voter],
        ) -> Result<(), crate::errors::OperationError> {
            Ok(())
        }
        async fn generate_state_transition(
            &self,
            _transition_data: StateTransitionData,
        ) -> Result<(Vec<node_data::ledger::SpentTransaction>, StateTransitionResult), crate::errors::OperationError>
        {
            Ok((
                vec![],
                StateTransitionResult {
                    state_root: [0u8; 32],
                    event_bloom: [0u8; 256],
                },
            ))
        }
        async fn add_step_elapsed_time(
            &self,
            _round: u64,
            _step_name: StepName,
            _elapsed: Duration,
        ) -> Result<(), crate::errors::OperationError> {
            Ok(())
        }
        async fn get_block_gas_limit(&self) -> u64 {
            0
        }
    }

    struct NoopHandler;

    #[async_trait]
    impl MsgHandler for NoopHandler {
        fn verify(
            &self,
            _msg: &Message,
            _round_committees: &crate::iteration_ctx::RoundCommittees,
        ) -> Result<(), ConsensusError> {
            Ok(())
        }

        async fn collect(
            &mut self,
            _msg: Message,
            _ru: &RoundUpdate,
            _committee: &crate::user::committee::Committee,
            _generator: Option<PublicKeyBytes>,
            _round_committees: &crate::iteration_ctx::RoundCommittees,
        ) -> Result<StepOutcome, ConsensusError> {
            Ok(StepOutcome::Pending)
        }

        async fn collect_from_past(
            &mut self,
            _msg: Message,
            _committee: &crate::user::committee::Committee,
            _generator: Option<PublicKeyBytes>,
        ) -> Result<StepOutcome, ConsensusError> {
            Ok(StepOutcome::Pending)
        }

        fn handle_timeout(
            &self,
            _ru: &RoundUpdate,
            _curr_iteration: u8,
        ) -> Option<Message> {
            None
        }
    }

    fn build_iter_ctx(
        db: Arc<Mutex<DummyDb>>,
    ) -> (
        IterationCtx<DummyDb>,
        SafeAttestationInfoRegistry,
        Arc<Mutex<ValidationHandler<DummyDb>>>,
    ) {
        let att_registry =
            Arc::new(Mutex::new(AttInfoRegistry::new()));
        let validation = Arc::new(Mutex::new(
            ValidationHandler::new(
                att_registry.clone(),
                db.clone(),
            ),
        ));
        let validation_ref = validation.clone();
        let ratification =
            Arc::new(Mutex::new(RatificationHandler::new(
                att_registry.clone(),
            )));
        let proposal =
            Arc::new(Mutex::new(ProposalHandler::new(db)));

        let mut timeouts: TimeoutSet = HashMap::new();
        timeouts.insert(StepName::Proposal, MIN_STEP_TIMEOUT);
        timeouts.insert(
            StepName::Validation,
            MIN_STEP_TIMEOUT + TIMEOUT_INCREASE,
        );
        timeouts.insert(
            StepName::Ratification,
            MIN_STEP_TIMEOUT + TIMEOUT_INCREASE + TIMEOUT_INCREASE,
        );

        (
            IterationCtx::new(1, 0, validation, ratification, proposal, timeouts),
            att_registry,
            validation_ref,
        )
    }

    fn build_candidate_message(ru: &RoundUpdate) -> Message {
        let mut header = Header::default();
        header.height = ru.round;
        header.iteration = 0;
        header.prev_block_hash = ru.hash();
        header.generator_bls_pubkey = *ru.pubkey_bls.bytes();
        header.txroot = merkle_root::<[u8; 32]>(&[]);
        header.faultroot = merkle_root::<[u8; 32]>(&[]);

        let block = Block::new(header, vec![], vec![]).expect("valid block");
        let mut candidate = Candidate { candidate: block };
        candidate.sign(&ru.secret_key, ru.pubkey_bls.inner());
        candidate.into()
    }

    fn build_quorum_message(
        ru: &RoundUpdate,
        iteration: u8,
        result: RatificationResult,
    ) -> Message {
        let att = Attestation {
            result,
            validation: StepVotes::default(),
            ratification: StepVotes::default(),
        };
        let header = ConsensusHeader {
            prev_block_hash: ru.hash(),
            round: ru.round,
            iteration,
        };
        let quorum = node_data::message::payload::Quorum { header, att };
        quorum.into()
    }

    #[tokio::test]
    async fn past_message_not_processed_outside_emergency() {
        let stored_candidates = Arc::new(Mutex::new(0usize));
        let db = Arc::new(Mutex::new(DummyDb {
            stored_candidates: stored_candidates.clone(),
            ..Default::default()
        }));
        let (mut iter_ctx, att_registry, validation_handler) = build_iter_ctx(db);

        let mut provisioners = Provisioners::empty();
        let mut rng = StdRng::seed_from_u64(1);
        let mut keys = Vec::new();
        for _ in 0..3 {
            let sk = BlsSecretKey::random(&mut rng);
            let pk = node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
            provisioners.add_provisioner_with_value(pk.clone(), 1000 * DUSK);
            keys.push((pk, sk));
        }

        let mut tip = node_data::ledger::Header::default();
        tip.height = 0;
        tip.seed = node_data::ledger::Seed::from([7u8; 48]);
        tip.hash = [3u8; 32];
        let (pk1, sk1) = &keys[0];
        let round_update = RoundUpdate::new(
            pk1.clone(),
            sk1.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let inbound = AsyncQueue::bounded(16, "test-inbound");
        let outbound = AsyncQueue::bounded(16, "test-outbound");
        let future_msgs = Arc::new(Mutex::new(MsgRegistry::default()));
        let client = Arc::new(DummyOps::default());

        iter_ctx.generate_iteration_committees(
            0,
            &provisioners,
            round_update.seed(),
        );

        let generator = iter_ctx
            .get_generator(0)
            .expect("generator for iter 0");
        let (gen_pk, gen_sk) = keys
            .iter()
            .find(|(pk, _)| pk.bytes() == &generator)
            .map(|(pk, sk)| (pk.clone(), sk.clone()))
            .expect("generator key");
        let generator_ru = RoundUpdate::new(
            gen_pk,
            gen_sk,
            &tip,
            HashMap::new(),
            vec![],
        );

        let mut ctx = ExecutionCtx::new(
            &mut iter_ctx,
            inbound,
            outbound,
            future_msgs,
            &provisioners,
            round_update,
            0,
            StepName::Validation,
            client,
            att_registry,
        );

        let msg = build_candidate_message(&generator_ru);
        let _ = ctx.process_inbound_msg(validation_handler, msg).await;

        let stored = *stored_candidates.lock().await;
        assert_eq!(
            stored, 0,
            "past messages should not be processed outside emergency"
        );
    }

    #[tokio::test]
    async fn future_message_is_queued_then_drained() {
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let (mut iter_ctx, att_registry, validation_handler) = build_iter_ctx(db);

        let mut provisioners = Provisioners::empty();
        let mut rng = StdRng::seed_from_u64(5);
        let mut keys = Vec::new();
        for _ in 0..3 {
            let sk = BlsSecretKey::random(&mut rng);
            let pk = node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
            provisioners.add_provisioner_with_value(pk.clone(), 1000 * DUSK);
            keys.push((pk, sk));
        }

        let mut tip = node_data::ledger::Header::default();
        tip.height = 0;
        tip.seed = node_data::ledger::Seed::from([7u8; 48]);
        tip.hash = [3u8; 32];
        let (pk1, sk1) = &keys[0];
        let round_update = RoundUpdate::new(
            pk1.clone(),
            sk1.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        iter_ctx.generate_iteration_committees(
            0,
            &provisioners,
            round_update.seed(),
        );
        iter_ctx.generate_iteration_committees(
            1,
            &provisioners,
            round_update.seed(),
        );

        let committee = iter_ctx
            .committees
            .get_committee(StepName::Validation.to_step(1))
            .expect("committee for iter 1");
        let member = committee.iter().next().expect("member");
        let (member_pk, member_sk) = keys
            .iter()
            .find(|(pk, _)| pk.bytes() == member.bytes())
            .map(|(pk, sk)| (pk.clone(), sk.clone()))
            .expect("member key");
        let signer_ru = RoundUpdate::new(
            member_pk,
            member_sk,
            &tip,
            HashMap::new(),
            vec![],
        );

        let inbound = AsyncQueue::bounded(16, "test-inbound");
        let outbound = AsyncQueue::bounded(16, "test-outbound");
        let future_msgs = Arc::new(Mutex::new(MsgRegistry::default()));
        let client = Arc::new(DummyOps::default());

        let mut ctx = ExecutionCtx::new(
            &mut iter_ctx,
            inbound.clone(),
            outbound.clone(),
            future_msgs.clone(),
            &provisioners,
            round_update.clone(),
            0,
            StepName::Validation,
            client.clone(),
            att_registry.clone(),
        );

        let future_validation =
            crate::validation::step::build_validation_payload(
                Vote::NoCandidate,
                &signer_ru,
                1,
            );
        let msg: Message = future_validation.into();
        let _ = ctx
            .process_inbound_msg(validation_handler.clone(), msg)
            .await;

        let queued = {
            let mut registry = future_msgs.lock().await;
            let step = StepName::Validation.to_step(1);
            let drained =
                registry.drain_msg_by_round_step(round_update.round, step);
            let len = drained.as_ref().map(|items| items.len()).unwrap_or(0);
            if let Some(items) = drained {
                for msg in items {
                    registry.put_msg(msg).expect("reinsert future msg");
                }
            }
            len
        };
        assert_eq!(queued, 1, "future message should be queued");

        validation_handler.lock().await.reset(1);
        let ctx_future = ExecutionCtx::new(
            &mut iter_ctx,
            inbound,
            outbound.clone(),
            future_msgs.clone(),
            &provisioners,
            round_update,
            1,
            StepName::Validation,
            client,
            att_registry,
        );

        let _ = ctx_future
            .handle_future_msgs(validation_handler)
            .await;

        let drained = future_msgs
            .lock()
            .await
            .drain_msg_by_round_step(
                tip.height + 1,
                StepName::Validation.to_step(1),
            )
            .map(|items| items.is_empty())
            .unwrap_or(true);
        assert!(drained, "future message should be drained");

        let forwarded = outbound.recv().await.expect("forwarded message");
        assert!(matches!(forwarded.payload, Payload::Validation(_)));
    }

    #[tokio::test]
    async fn open_consensus_mode_broadcasts_only_success_quorums() {
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let att_registry =
            Arc::new(Mutex::new(AttInfoRegistry::new()));
        let validation = Arc::new(Mutex::new(
            ValidationHandler::new(att_registry.clone(), db.clone()),
        ));
        let ratification = Arc::new(Mutex::new(
            RatificationHandler::new(att_registry.clone()),
        ));
        let proposal =
            Arc::new(Mutex::new(ProposalHandler::new(db)));

        let mut timeouts: TimeoutSet = HashMap::new();
        timeouts.insert(StepName::Proposal, Duration::from_millis(20));
        timeouts.insert(StepName::Validation, Duration::from_millis(20));
        timeouts.insert(StepName::Ratification, Duration::from_millis(20));

        let mut iter_ctx = IterationCtx::new(
            1,
            0,
            validation,
            ratification,
            proposal,
            timeouts,
        );

        let mut provisioners = Provisioners::empty();
        let mut rng = StdRng::seed_from_u64(11);
        let mut keys = Vec::new();
        for _ in 0..3 {
            let sk = BlsSecretKey::random(&mut rng);
            let pk =
                node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
            provisioners.add_provisioner_with_value(pk.clone(), 1000 * DUSK);
            keys.push((pk, sk));
        }

        let mut tip = Header::default();
        tip.height = 0;
        tip.seed = node_data::ledger::Seed::from([7u8; 48]);
        tip.hash = [3u8; 32];
        let (pk1, sk1) = &keys[0];
        let round_update = RoundUpdate::new(
            pk1.clone(),
            sk1.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let iter = CONSENSUS_MAX_ITER - 1;
        iter_ctx.generate_iteration_committees(
            iter,
            &provisioners,
            round_update.seed(),
        );

        let inbound = AsyncQueue::bounded(16, "test-inbound");
        let outbound = AsyncQueue::bounded(16, "test-outbound");
        let future_msgs = Arc::new(Mutex::new(MsgRegistry::default()));
        let client = Arc::new(DummyOps::default());

        let mut ctx = ExecutionCtx::new(
            &mut iter_ctx,
            inbound.clone(),
            outbound.clone(),
            future_msgs,
            &provisioners,
            round_update.clone(),
            iter,
            StepName::Ratification,
            client,
            att_registry,
        );

        let phase = Arc::new(Mutex::new(NoopHandler));
        let mut event_loop = Box::pin(ctx.event_loop(phase, None));

        let driver = async {
            tokio::time::sleep(Duration::from_millis(80)).await;

            let fail_quorum = build_quorum_message(
                &round_update,
                iter,
                RatificationResult::Fail(Vote::NoCandidate),
            );
            inbound.try_send(fail_quorum);

            let fail_forwarded = tokio::time::timeout(
                Duration::from_millis(60),
                outbound.recv(),
            )
            .await;
            assert!(
                fail_forwarded.is_err(),
                "fail quorum should not be rebroadcast in open consensus"
            );

            let success_quorum = build_quorum_message(
                &round_update,
                iter,
                RatificationResult::Success(Vote::Valid([9u8; 32])),
            );
            inbound.try_send(success_quorum);

            let forwarded = tokio::time::timeout(
                Duration::from_millis(200),
                outbound.recv(),
            )
            .await
            .expect("expected success quorum")
            .expect("outbound message");

            match forwarded.payload {
                Payload::Quorum(q) => match q.att.result {
                    RatificationResult::Success(_) => {}
                    other => panic!("unexpected quorum result: {other:?}"),
                },
                _ => panic!("expected quorum payload"),
            }
        };

        tokio::select! {
            _ = &mut event_loop => {
                panic!("open consensus loop should not exit early");
            }
            _ = driver => {}
        }
    }

    #[tokio::test]
    async fn invalid_validation_quorum_is_rejected() {
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let (mut iter_ctx, att_registry, validation_handler) =
            build_iter_ctx(db.clone());

        let mut provisioners = Provisioners::empty();
        let mut rng = StdRng::seed_from_u64(12);
        let mut keys = Vec::new();
        for _ in 0..3 {
            let sk = BlsSecretKey::random(&mut rng);
            let pk =
                node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
            provisioners.add_provisioner_with_value(pk.clone(), 1000 * DUSK);
            keys.push((pk, sk));
        }

        let mut tip = Header::default();
        tip.height = 0;
        tip.seed = node_data::ledger::Seed::from([7u8; 48]);
        tip.hash = [3u8; 32];
        let (pk1, sk1) = &keys[0];
        let round_update = RoundUpdate::new(
            pk1.clone(),
            sk1.clone(),
            &tip,
            HashMap::new(),
            vec![],
        );

        let msg_iter = crate::config::EMERGENCY_MODE_ITERATION_THRESHOLD;
        let current_iter = msg_iter + 1;
        iter_ctx.generate_iteration_committees(
            msg_iter,
            &provisioners,
            round_update.seed(),
        );
        iter_ctx.generate_iteration_committees(
            current_iter,
            &provisioners,
            round_update.seed(),
        );

        let inbound = AsyncQueue::bounded(16, "test-inbound");
        let outbound = AsyncQueue::bounded(16, "test-outbound");
        let future_msgs = Arc::new(Mutex::new(MsgRegistry::default()));
        let client = Arc::new(DummyOps::default());

        let mut ctx = ExecutionCtx::new(
            &mut iter_ctx,
            inbound,
            outbound.clone(),
            future_msgs,
            &provisioners,
            round_update,
            current_iter,
            StepName::Validation,
            client,
            att_registry,
        );

        let header = ConsensusHeader {
            prev_block_hash: tip.hash,
            round: tip.height + 1,
            iteration: msg_iter,
        };
        let bad_votes = StepVotes::new([0u8; 48], 1);
        let validation = ValidationResult::new(
            bad_votes,
            Vote::Valid([1u8; 32]),
            QuorumType::Valid,
        );
        let vq = ValidationQuorum { header, result: validation };
        let msg: Message = vq.into();

        let _ = ctx
            .process_inbound_msg(validation_handler, msg)
            .await;

        let stored = db.lock().await.validation_results.lock().await.len();
        assert_eq!(
            stored, 0,
            "invalid validation quorum should be rejected"
        );

        let forwarded = tokio::time::timeout(
            Duration::from_millis(100),
            outbound.recv(),
        )
        .await;
        assert!(
            forwarded.is_err(),
            "invalid validation quorum should not be rebroadcast"
        );
    }
}

#[inline(always)]
fn log_msg(event: &str, src: &str, msg: &Message) {
    debug!(
        event,
        src,
        topic = ?msg.topic(),
        info = ?msg.header,
        ray_id = msg.ray_id()
    );
}
