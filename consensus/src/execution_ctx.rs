// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, QuorumMsgSender, RoundUpdate};
use crate::config::CONSENSUS_MAX_TIMEOUT_MS;
use crate::contract_state::Operations;
use crate::msg_handler::HandleMsgOutput::{Ready, ReadyWithTimeoutIncrease};
use crate::msg_handler::MsgHandler;
use crate::queue::Queue;
use crate::step_votes_reg::SafeCertificateInfoRegistry;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::{proposal, ratification, validation};
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{to_str, Block};
use node_data::message::Payload;
use node_data::message::{AsyncQueue, Message, Topics};
use node_data::StepName;
use std::cmp;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};

/// A pool of all generated committees
#[derive(Default)]
pub struct RoundCommittees {
    committees: HashMap<u8, Committee>,
}

impl RoundCommittees {
    pub(crate) fn get_committee(&self, step: u8) -> Option<&Committee> {
        self.committees.get(&step)
    }

    pub(crate) fn get_generator(&self, iter: u8) -> Option<PublicKeyBytes> {
        let step = StepName::Proposal.to_step(iter);
        self.get_committee(step)
            .and_then(|c| c.iter().next().map(|p| *p.bytes()))
    }

    pub(crate) fn get_validation_committee(
        &self,
        iter: u8,
    ) -> Option<&Committee> {
        let step = StepName::Validation.to_step(iter);
        self.get_committee(step)
    }

    pub(crate) fn insert(&mut self, step: u8, committee: Committee) {
        self.committees.insert(step, committee);
    }
}

/// Represents a shared state within a context of the exection of a single
/// iteration.
pub struct IterationCtx<DB: Database> {
    validation_handler: Arc<Mutex<validation::handler::ValidationHandler>>,
    ratification_handler:
        Arc<Mutex<ratification::handler::RatificationHandler>>,
    proposal_handler: Arc<Mutex<proposal::handler::ProposalHandler<DB>>>,

    pub join_set: JoinSet<()>,

    round: u64,
    iter: u8,

    /// Stores any committee already generated in the execution of any
    /// iteration of current round
    committees: RoundCommittees,
}

impl<D: Database> IterationCtx<D> {
    pub fn new(
        round: u64,
        iter: u8,
        proposal_handler: Arc<Mutex<proposal::handler::ProposalHandler<D>>>,
        validation_handler: Arc<Mutex<validation::handler::ValidationHandler>>,
        ratification_handler: Arc<
            Mutex<ratification::handler::RatificationHandler>,
        >,
    ) -> Self {
        Self {
            round,
            join_set: JoinSet::new(),
            iter,
            proposal_handler,
            validation_handler,
            ratification_handler,
            committees: Default::default(),
        }
    }

    pub(crate) async fn collect_past_event(
        &self,
        ru: &RoundUpdate,
        msg: &Message,
    ) -> Option<Message> {
        let committee = self.committees.get_committee(msg.header.get_step())?;
        match msg.topic() {
            node_data::message::Topics::Candidate => {
                let mut handler = self.proposal_handler.lock().await;
                _ = handler
                    .collect_from_past(
                        msg.clone(),
                        ru,
                        msg.header.iteration,
                        committee,
                    )
                    .await;
            }
            node_data::message::Topics::Validation => {
                let mut handler = self.validation_handler.lock().await;
                if let Ok(Ready(m)) = handler
                    .collect_from_past(
                        msg.clone(),
                        ru,
                        msg.header.iteration,
                        committee,
                    )
                    .await
                {
                    return Some(m);
                }
            }
            node_data::message::Topics::Ratification => {
                let mut handler = self.ratification_handler.lock().await;
                if let Ok(Ready(m)) = handler
                    .collect_from_past(
                        msg.clone(),
                        ru,
                        msg.header.iteration,
                        committee,
                    )
                    .await
                {
                    return Some(m);
                }
            }
            _ => {}
        };

        None
    }

    pub(crate) fn on_begin(&mut self, iter: u8) {
        self.iter = iter;
    }

    pub(crate) fn get_generator(&self, iter: u8) -> Option<PublicKeyBytes> {
        let step = StepName::Proposal.to_step(iter);
        self.committees
            .get_committee(step)
            .and_then(|c| c.iter().next().map(|p| *p.bytes()))
    }

    pub(crate) fn on_end(&mut self) {
        debug!(
            event = "iter completed",
            len = self.join_set.len(),
            round = self.round,
            iter = self.iter,
        );
        self.join_set.abort_all();
    }
}

impl<DB: Database> Drop for IterationCtx<DB> {
    fn drop(&mut self) {
        self.on_end();
    }
}

/// ExecutionCtx encapsulates all data needed by a single step to be fully
/// executed.
pub struct ExecutionCtx<'a, DB: Database, T> {
    pub iter_ctx: &'a mut IterationCtx<DB>,

    /// Messaging-related fields
    pub inbound: AsyncQueue<Message>,
    pub outbound: AsyncQueue<Message>,
    pub future_msgs: Arc<Mutex<Queue<Message>>>,

    /// State-related fields
    pub provisioners: &'a Provisioners,

    // Round/Step parameters
    pub round_update: RoundUpdate,
    pub iteration: u8,
    pub step: StepName,

    _executor: Arc<Mutex<T>>,

    pub sv_registry: SafeCertificateInfoRegistry,
    quorum_sender: QuorumMsgSender,
}

impl<'a, DB: Database, T: Operations + 'static> ExecutionCtx<'a, DB, T> {
    /// Creates step execution context.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        iter_ctx: &'a mut IterationCtx<DB>,
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        future_msgs: Arc<Mutex<Queue<Message>>>,
        provisioners: &'a Provisioners,
        round_update: RoundUpdate,
        iteration: u8,
        step: StepName,
        executor: Arc<Mutex<T>>,
        sv_registry: SafeCertificateInfoRegistry,
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
            _executor: executor,
            sv_registry,
            quorum_sender,
        }
    }

    pub fn total_step(&self) -> u8 {
        self.step.to_step(self.iteration)
    }

    /// Returns true if `my pubkey` is a member of [`committee`].
    pub(crate) fn am_member(&self, committee: &Committee) -> bool {
        committee.is_member(&self.round_update.pubkey_bls)
    }

    pub(crate) fn save_committee(&mut self, committee: Committee) {
        self.iter_ctx
            .committees
            .insert(self.total_step(), committee);
    }

    pub(crate) fn get_current_committee(&self) -> Option<&Committee> {
        self.iter_ctx.committees.get_committee(self.total_step())
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
    pub async fn event_loop<C: MsgHandler<Message>>(
        &mut self,
        phase: Arc<Mutex<C>>,
        timeout_millis: &mut u64,
    ) -> Result<Message, ConsensusError> {
        debug!(event = "run event_loop");

        // Calculate timeout
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(*timeout_millis))
            .unwrap();

        let inbound = self.inbound.clone();

        // Handle both timeout event and messages from inbound queue.
        loop {
            match time::timeout_at(deadline, inbound.recv()).await {
                // Inbound message event
                Ok(result) => {
                    if let Ok(msg) = result {
                        if let Some(step_result) = self
                            .process_inbound_msg(
                                phase.clone(),
                                msg,
                                timeout_millis,
                            )
                            .await
                        {
                            return Ok(step_result);
                        }
                    }
                }
                // Timeout event. Phase could not reach its final goal.
                // Increase timeout for next execution of this step and move on.
                Err(_) => {
                    info!(event = "timeout-ed");
                    Self::increase_timeout(timeout_millis);

                    return self.process_timeout_event(phase.clone()).await;
                }
            }
        }
    }

    pub(crate) async fn vote_for_former_candidate(
        &mut self,
        msg_step: u8,
        candidate: &Block,
    ) {
        debug!(
            event = "former candidate received",
            hash = to_str(&candidate.header().hash),
            msg_step,
        );

        if msg_step < self.total_step() {
            self.try_vote(msg_step + 1, candidate, Topics::Validation);
        }

        if msg_step + 2 <= self.total_step() {
            self.try_vote(msg_step + 2, candidate, Topics::Ratification);
        }
    }

    fn try_vote(&mut self, msg_step: u8, candidate: &Block, topic: Topics) {
        if let Some(committee) =
            self.iter_ctx.committees.get_committee(msg_step)
        {
            if self.am_member(committee) {
                debug!(
                    event = "vote for former candidate",
                    step_topic = format!("{:?}", topic),
                    hash = to_str(&candidate.header().hash),
                    msg_step,
                );

                // TODO: Verify
            };
        } else {
            error!(
                event = "committee not found",
                step = self.total_step(),
                msg_step
            );
        }
    }

    /// Process messages from past
    async fn process_past_events(&mut self, msg: &Message) -> Option<Message> {
        if msg.header.round != self.round_update.round {
            return None;
        }

        if let Err(e) = self.outbound.send(msg.clone()).await {
            error!("could not send msg due to {:?}", e);
        }

        // Try to vote for candidate block from former iteration
        if let Payload::Candidate(p) = &msg.payload {
            // TODO: Perform block header/ Certificate full verification
            // To be addressed with another PR

            self.vote_for_former_candidate(msg.header.get_step(), &p.candidate)
                .await;
        }

        // Collect message from a previous reduction step/iteration.
        if let Some(m) = self
            .iter_ctx
            .collect_past_event(&self.round_update, msg)
            .await
        {
            if m.header.topic == Topics::Quorum {
                debug!(
                    event = "quorum",
                    src = "prev_step",
                    msg_step = m.header.get_step(),
                    hash = to_str(&m.header.block_hash),
                );

                self.quorum_sender.send(m).await;
            }
        }

        None
    }

    /// Delegates the received message to the Phase handler for further
    /// processing.
    ///
    /// Returning Option::Some here is interpreted as FinalMessage by
    /// event_loop.
    async fn process_inbound_msg<C: MsgHandler<Message>>(
        &mut self,
        phase: Arc<Mutex<C>>,
        msg: Message,
        timeout_millis: &mut u64,
    ) -> Option<Message> {
        let committee = self
            .get_current_committee()
            .expect("committee to be created before run");
        // Check if message is valid in the context of current step
        let ret = phase.lock().await.is_valid(
            &msg,
            &self.round_update,
            self.iteration,
            self.step,
            committee,
            &self.iter_ctx.committees,
        );

        match ret {
            Ok(_) => {
                // Re-publish the returned message
                self.outbound.send(msg.clone()).await.unwrap_or_else(|err| {
                    error!("unable to re-publish a handled msg {:?}", err)
                });
            }
            // An error here means an phase considers this message as invalid.
            // This could be due to failed verification, bad round/step.
            Err(e) => {
                match e {
                    ConsensusError::FutureEvent => {
                        trace!("future msg {:?}", msg);
                        // This is a message from future round or step.
                        // Save it in future_msgs to be processed when we reach
                        // same round/step.
                        self.future_msgs.lock().await.put_event(
                            msg.header.round,
                            msg.header.get_step(),
                            msg,
                        );

                        return None;
                    }
                    ConsensusError::PastEvent => {
                        return self.process_past_events(&msg).await;
                    }
                    _ => {
                        error!("phase handler err: {:?}", e);
                        return None;
                    }
                }
            }
        }

        let ret = phase
            .lock()
            .await
            .collect(
                msg.clone(),
                &self.round_update,
                self.iteration,
                self.step,
                committee,
            )
            .await;

        match ret {
            Ok(output) => {
                trace!("message collected {:#?}", msg);

                match output {
                    Ready(m) => {
                        // Fully valid state reached on this step. Return it as
                        // an output to populate next step with it.
                        return Some(m);
                    }
                    ReadyWithTimeoutIncrease(m) => {
                        Self::increase_timeout(timeout_millis);
                        return Some(m);
                    }
                    _ => {} /* Message collected but phase does not reach
                             * a final result */
                }
            }
            Err(e) => {
                error!(
                    event = "failed collect",
                    err = format!("{:?}", e),
                    msg_topic = format!("{:?}", msg.topic()),
                    msg_iter = msg.header.iteration,
                    msg_step = msg.header.get_step(),
                    msg_round = msg.header.round,
                );
            }
        }

        None
    }

    /// Delegates the received event of timeout to the Phase handler for further
    /// processing.
    async fn process_timeout_event<C: MsgHandler<Message>>(
        &mut self,
        phase: Arc<Mutex<C>>,
    ) -> Result<Message, ConsensusError> {
        if let Ok(Ready(msg)) = phase
            .lock()
            .await
            .handle_timeout(&self.round_update, self.iteration)
        {
            return Ok(msg);
        }

        Ok(Message::empty())
    }

    /// Handles all messages stored in future_msgs queue that belongs to the
    /// current round and step.
    ///
    /// Returns Some(msg) if the step is finalized.
    pub async fn handle_future_msgs<C: MsgHandler<Message>>(
        &self,
        phase: Arc<Mutex<C>>,
    ) -> Option<Message> {
        let committee = self
            .get_current_committee()
            .expect("committee to be created before run");
        if let Some(messages) = self
            .future_msgs
            .lock()
            .await
            .drain_events(self.round_update.round, self.total_step())
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
                    debug!(
                        event = "republish",
                        src = "future_msgs",
                        msg_step = msg.header.get_step(),
                        msg_round = msg.header.round,
                        msg_topic = ?msg.header.topic,
                    );

                    self.outbound.send(msg.clone()).await.unwrap_or_else(
                        |err| {
                            error!(
                                "unable to re-publish a drained msg {:?}",
                                err
                            )
                        },
                    );

                    if let Ok(Ready(msg)) = phase
                        .lock()
                        .await
                        .collect(
                            msg,
                            &self.round_update,
                            self.iteration,
                            self.step,
                            committee,
                        )
                        .await
                    {
                        return Some(msg);
                    }
                }
            }
        }

        None
    }

    pub fn get_sortition_config(
        &self,
        size: usize,
        exclusion: Option<PublicKeyBytes>,
    ) -> sortition::Config {
        sortition::Config::new(
            self.round_update.seed(),
            self.round_update.round,
            self.total_step(),
            size,
            exclusion,
        )
    }

    fn increase_timeout(timeout_millis: &mut u64) {
        // Increase timeout up to CONSENSUS_MAX_TIMEOUT_MS
        *timeout_millis =
            cmp::min(*timeout_millis * 2, CONSENSUS_MAX_TIMEOUT_MS);
    }
}
