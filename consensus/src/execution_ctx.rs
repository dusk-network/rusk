// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Database;
use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::HandleMsgOutput::{
    FinalResult, FinalResultWithTimeoutIncrease,
};
use crate::msg_handler::MsgHandler;
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::{firststep, secondstep, selection};
use node_data::message::{AsyncQueue, Message, Topics};
use std::cmp;
use tokio::task::JoinSet;

use crate::config::CONSENSUS_MAX_TIMEOUT_MS;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};

/// Represents a shared state within a context of the exection of a single
/// iteration.
pub struct IterationCtx<DB: Database> {
    first_reduction_handler: Arc<Mutex<firststep::handler::Reduction<DB>>>,
    sec_reduction_handler: Arc<Mutex<secondstep::handler::Reduction>>,
    selection_handler: Arc<Mutex<selection::handler::Selection<DB>>>,

    pub join_set: JoinSet<()>,

    /// verified candidate hash
    ///
    /// An optimization to call VST once per a candidate block when this
    /// provisioner is extracted for both reductions.
    pub verified_hash: Arc<Mutex<[u8; 32]>>,

    round: u64,
    iter: u8,
}

impl<D: Database> IterationCtx<D> {
    pub fn new(
        round: u64,
        iter: u8,
        selection_handler: Arc<Mutex<selection::handler::Selection<D>>>,
        first_reduction_handler: Arc<Mutex<firststep::handler::Reduction<D>>>,
        sec_reduction_handler: Arc<Mutex<secondstep::handler::Reduction>>,
    ) -> Self {
        Self {
            round,
            join_set: JoinSet::new(),
            iter,
            verified_hash: Arc::new(Mutex::new([0u8; 32])),
            selection_handler,
            first_reduction_handler,
            sec_reduction_handler,
        }
    }

    pub(crate) async fn collect_past_event(
        &self,
        ru: &RoundUpdate,
        msg: &Message,
    ) -> Option<Message> {
        // TODO: call verify() and then re-publish

        match msg.topic() {
            node_data::message::Topics::NewBlock => {
                let mut handler = self.selection_handler.lock().await;

                _ = handler
                    .collect(
                        msg.clone(),
                        ru,
                        msg.header.step,
                        &Committee::default(),
                    )
                    .await;
            }
            node_data::message::Topics::FirstReduction => {
                let mut handler = self.first_reduction_handler.lock().await;

                if let Ok(FinalResult(m)) = handler
                    .collect(
                        msg.clone(),
                        ru,
                        msg.header.step,
                        &Committee::default(),
                    )
                    .await
                {
                    // Fully valid state reached on this step. Return it
                    // as an output to
                    // populate next step with it.
                    return Some(m);
                }
            }
            node_data::message::Topics::SecondReduction => {
                let mut handler = self.sec_reduction_handler.lock().await;

                if let Ok(FinalResult(m)) = handler
                    .collect(
                        msg.clone(),
                        ru,
                        msg.header.step,
                        &Committee::default(),
                    )
                    .await
                {
                    // Fully valid state reached on this step. Return it
                    // as an output to
                    // populate next step with it.
                    return Some(m);
                }
            }
            _ => {}
        };

        None
    }
}

impl<DB: Database> Drop for IterationCtx<DB> {
    fn drop(&mut self) {
        debug!(
            event = "iter completed",
            len = self.join_set.len(),
            round = self.round,
            iter = self.iter,
        );
        self.join_set.abort_all();
    }
}

/// ExecutionCtx encapsulates all data needed by a single step to be fully
/// executed.
pub struct ExecutionCtx<'a, DB: Database> {
    pub iter_ctx: &'a mut IterationCtx<DB>,

    /// Messaging-related fields
    pub inbound: AsyncQueue<Message>,
    pub outbound: AsyncQueue<Message>,
    pub future_msgs: Arc<Mutex<Queue<Message>>>,

    /// State-related fields
    pub provisioners: &'a mut Provisioners,

    // Round/Step parameters
    pub round_update: RoundUpdate,
    pub step: u8,
}

impl<'a, DB: Database> ExecutionCtx<'a, DB> {
    /// Creates step execution context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        iter_ctx: &'a mut IterationCtx<DB>,
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        future_msgs: Arc<Mutex<Queue<Message>>>,
        provisioners: &'a mut Provisioners,
        round_update: RoundUpdate,
        step: u8,
    ) -> Self {
        Self {
            iter_ctx,
            inbound,
            outbound,
            future_msgs,
            provisioners,
            round_update,
            step,
        }
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
        committee: &Committee,
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
                                committee,
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

    /// Process messages from past
    async fn process_past_events(&mut self, msg: &Message) -> Option<Message> {
        if msg.header.block_hash == [0u8; 32]
            || msg.header.round != self.round_update.round
        {
            return None;
        }

        if let Err(e) = self.outbound.send(msg.clone()).await {
            error!("could not send newblock msg due to {:?}", e);
        }

        if let Some(m) = self
            .iter_ctx
            .collect_past_event(&self.round_update, msg)
            .await
        {
            if m.header.topic == Topics::Agreement as u8 {
                debug!(
                    event = "agreement from previous iter",
                    msg_step = m.header.step
                );
                return Some(m);
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
        committee: &Committee,
        phase: Arc<Mutex<C>>,
        msg: Message,
        timeout_millis: &mut u64,
    ) -> Option<Message> {
        // Check if a message is fully valid. If so, then it can be broadcast.
        match phase.lock().await.is_valid(
            msg.clone(),
            &self.round_update,
            self.step,
            committee,
        ) {
            Ok(msg) => {
                // Re-publish the returned message
                self.outbound.send(msg).await.unwrap_or_else(|err| {
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
                            msg.header.step,
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

        match phase
            .lock()
            .await
            .collect(msg.clone(), &self.round_update, self.step, committee)
            .await
        {
            Ok(output) => {
                trace!("message collected {:#?}", msg);

                match output {
                    FinalResult(m) => {
                        // Fully valid state reached on this step. Return it as
                        // an output to populate next step with it.
                        return Some(m);
                    }
                    FinalResultWithTimeoutIncrease(m) => {
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
                    msg_step = msg.header.step,
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
        let mut phase = phase.lock().await;

        if let Ok(FinalResult(msg)) =
            phase.handle_timeout(&self.round_update, self.step)
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
        committee: &Committee,
        phase: Arc<Mutex<C>>,
    ) -> Option<Message> {
        let mut phase = phase.lock().await;

        if let Some(messages) = self
            .future_msgs
            .lock()
            .await
            .drain_events(self.round_update.round, self.step)
        {
            if !messages.is_empty() {
                debug!(event = "drain future msgs", count = messages.len(),)
            }

            for msg in messages {
                if let Ok(msg) = phase.is_valid(
                    msg,
                    &self.round_update,
                    self.step,
                    committee,
                ) {
                    if let Ok(FinalResult(msg)) = phase
                        .collect(msg, &self.round_update, self.step, committee)
                        .await
                    {
                        return Some(msg);
                    }
                }
            }
        }

        None
    }

    pub fn get_sortition_config(&self, size: usize) -> sortition::Config {
        sortition::Config::new(
            self.round_update.seed,
            self.round_update.round,
            self.step,
            size,
        )
    }

    fn increase_timeout(timeout_millis: &mut u64) {
        // Increase timeout up to CONSENSUS_MAX_TIMEOUT_MS
        *timeout_millis =
            cmp::min(*timeout_millis * 2, CONSENSUS_MAX_TIMEOUT_MS);
    }
}
