// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use node_data::message::{AsyncQueue, Message};
use std::cmp;

use crate::config::CONSENSUS_MAX_TIMEOUT_MS;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::{error, trace};

/// ExecutionCtx encapsulates all data needed by a single step to be fully
/// executed.
pub struct ExecutionCtx<'a> {
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

impl<'a> ExecutionCtx<'a> {
    /// Creates step execution context.
    pub fn new(
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        future_msgs: Arc<Mutex<Queue<Message>>>,
        provisioners: &'a mut Provisioners,
        round_update: RoundUpdate,
        step: u8,
    ) -> Self {
        Self {
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
        phase: &mut C,
        timeout_millis: &mut u64,
    ) -> Result<Message, ConsensusError> {
        tracing::info!("event: run event_loop");

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
                            .process_inbound_msg(committee, phase, msg)
                            .await
                        {
                            return Ok(step_result);
                        }
                    }
                }
                // Timeout event
                Err(_) => {
                    tracing::info!("event: timeout");

                    // Increase timeout up to CONSENSUS_MAX_TIMEOUT_MS
                    *timeout_millis =
                        cmp::min(*timeout_millis * 2, CONSENSUS_MAX_TIMEOUT_MS);

                    return self.process_timeout_event(phase);
                }
            }
        }
    }

    /// Delegates the received message to the Phase handler for further
    /// processing.
    ///
    /// Returning Option::Some here is interpreted as FinalMessage by
    /// event_loop.
    async fn process_inbound_msg<C: MsgHandler<Message>>(
        &mut self,
        committee: &Committee,
        phase: &mut C,
        msg: Message,
    ) -> Option<Message> {
        // Check if a message is fully valid. If so, then it can be broadcast.
        match phase.is_valid(
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
                    }
                    ConsensusError::PastEvent => {
                        trace!("discard message from past {:?}", msg);
                    }
                    _ => {
                        error!("phase handler err: {:?}", e);
                    }
                }

                return None;
            }
        }

        match phase
            .collect(msg.clone(), &self.round_update, self.step, committee)
            .await
        {
            // Fully valid state reached on this step. Return it as an output.
            // Populate next step with it.
            Ok(output) => {
                trace!("message collected {:?}", msg);

                if let HandleMsgOutput::FinalResult(msg) = output {
                    return Some(msg);
                }
            }
            Err(e) => {
                error!("phase collect return err: {:?}", e);
            }
        }

        None
    }

    /// Delegates the received event of timeout to the Phase handler for further
    /// processing.
    fn process_timeout_event<C: MsgHandler<Message>>(
        &mut self,
        phase: &mut C,
    ) -> Result<Message, ConsensusError> {
        if let Ok(HandleMsgOutput::FinalResult(msg)) =
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
        phase: &mut C,
    ) -> Option<Message> {
        if let Some(messages) = self
            .future_msgs
            .lock()
            .await
            .drain_events(self.round_update.round, self.step)
        {
            for msg in messages {
                if let Ok(msg) = phase.is_valid(
                    msg,
                    &self.round_update,
                    self.step,
                    committee,
                ) {
                    if let Ok(HandleMsgOutput::FinalResult(msg)) = phase
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
}
