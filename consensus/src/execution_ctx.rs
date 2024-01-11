// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, QuorumMsgSender, RoundUpdate};

use crate::contract_state::Operations;
use crate::iteration_ctx::IterationCtx;
use crate::msg_handler::HandleMsgOutput::{Pending, Ready};
use crate::msg_handler::MsgHandler;
use crate::queue::Queue;
use crate::step_votes_reg::SafeCertificateInfoRegistry;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::{to_str, Block};
use node_data::message::Payload;
use node_data::message::{AsyncQueue, Message, Topics};

use node_data::StepName;

use crate::config::EMERGENCY_MODE_ITERATION_THRESHOLD;
use crate::validation::step::ValidationStep;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};

/// ExecutionCtx encapsulates all data needed in the execution of consensus
/// messages handlers.
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
    step: StepName,

    executor: Arc<Mutex<T>>,

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
            executor,
            sv_registry,
            quorum_sender,
        }
    }

    pub fn step_name(&self) -> StepName {
        self.step
    }

    pub fn step(&self) -> u16 {
        self.step.to_step(self.iteration)
    }

    /// Returns true if `my pubkey` is a member of [`committee`].
    pub(crate) fn am_member(&self, committee: &Committee) -> bool {
        committee.is_member(&self.round_update.pubkey_bls)
    }

    pub(crate) fn save_committee(&mut self, committee: Committee) {
        self.iter_ctx.committees.insert(self.step(), committee);
    }

    pub(crate) fn get_current_committee(&self) -> Option<&Committee> {
        self.iter_ctx.committees.get_committee(self.step())
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
    ) -> Result<Message, ConsensusError> {
        debug!(event = "run event_loop");

        let timeout = self.iter_ctx.get_timeout(self.step_name());
        let deadline = Instant::now().checked_add(timeout).unwrap();

        let inbound = self.inbound.clone();

        // Handle both timeout event and messages from inbound queue.
        loop {
            match time::timeout_at(deadline, inbound.recv()).await {
                // Inbound message event
                Ok(result) => {
                    if let Ok(msg) = result {
                        if let Some(step_result) =
                            self.process_inbound_msg(phase.clone(), msg).await
                        {
                            return Ok(step_result);
                        }
                    }
                }
                // Timeout event. Phase could not reach its final goal.
                // Increase timeout for next execution of this step and move on.
                Err(_) => {
                    info!(event = "timeout-ed");

                    return self.process_timeout_event(phase.clone()).await;
                }
            }
        }
    }

    /// Cast a validation vote for a candidate that originates from former
    /// iteration
    pub(crate) async fn try_cast_validation_vote(
        &mut self,
        candidate_msg_step: u16,
        candidate: &Block,
    ) {
        let validation_step = candidate_msg_step + 1;
        if let Some(committee) =
            self.iter_ctx.committees.get_committee(validation_step)
        {
            if self.am_member(committee) {
                ValidationStep::try_vote(
                    candidate,
                    &self.round_update,
                    (candidate_msg_step / 3) as u8,
                    self.outbound.clone(),
                    self.inbound.clone(),
                    self.executor.clone(),
                )
                .await;
            };
        } else {
            error!(event = "committee not found", validation_step);
        }
    }

    /// Process messages from past
    async fn process_past_events(&mut self, msg: Message) -> Option<Message> {
        if msg.header.round != self.round_update.round
            || self.iteration < EMERGENCY_MODE_ITERATION_THRESHOLD
        {
            return None;
        }

        self.on_emergency_mode(msg).await
    }

    /// Handles a consensus message in emergency mode
    async fn on_emergency_mode(&mut self, msg: Message) -> Option<Message> {
        if let Err(e) = self.outbound.send(msg.clone()).await {
            error!("could not send msg due to {:?}", e);
        }

        // Try to vote for candidate block from former iteration
        if let Payload::Candidate(p) = &msg.payload {
            self.try_cast_validation_vote(msg.header.get_step(), &p.candidate)
                .await;

            // TODO: try_cast_ratification_vote
        }

        // Collect message from a previous iteration/step.
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
    ) -> Option<Message> {
        let committee = self
            .get_current_committee()
            .expect("committee to be created before run");
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
                        return self.process_past_events(msg).await;
                    }
                    _ => {
                        error!("phase handler err: {:?}", e);
                        return None;
                    }
                }
            }
        }

        let msg_topic = msg.topic();
        let msg_iter = msg.header.iteration;
        let msg_step = msg.header.get_step();
        let msg_round = msg.header.round;
        trace!("collecting msg {msg:#?}");

        let collected = phase
            .lock()
            .await
            .collect(msg, &self.round_update, committee)
            .await;

        match collected {
            // Fully valid state reached on this step. Return it as an output to
            // populate next step with it.
            Ok(Ready(m)) => Some(m),
            // Message collected but phase didn't reach a final result
            Ok(Pending(_)) => None,
            Err(err) => {
                let event = "failed collect";
                error!(event, ?err, ?msg_topic, msg_iter, msg_step, msg_round,);
                None
            }
        }
    }

    /// Delegates the received event of timeout to the Phase handler for further
    /// processing.
    async fn process_timeout_event<C: MsgHandler<Message>>(
        &mut self,
        phase: Arc<Mutex<C>>,
    ) -> Result<Message, ConsensusError> {
        self.iter_ctx.on_timeout_event();

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
            .drain_events(self.round_update.round, self.step())
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
                        .collect(msg, &self.round_update, committee)
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
        exclusion: Option<PublicKeyBytes>,
    ) -> sortition::Config {
        sortition::Config::new(
            self.round_update.seed(),
            self.round_update.round,
            self.iteration,
            self.step_name(),
            exclusion,
        )
    }
}
