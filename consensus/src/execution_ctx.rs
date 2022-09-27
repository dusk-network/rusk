// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate, SelectError};
use crate::messages::Message;
use crate::msg_handler::MsgHandler;
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::util::pending_queue::PendingQueue;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time;
use tokio::time::Instant;
use tracing::warn;

/// ExecutionCtx encapsulates all data needed by a single step to be fully executed.
pub struct ExecutionCtx<'a> {
    pub cancel_chan: &'a mut oneshot::Receiver<bool>,

    /// Messaging-related fields
    pub inbound: PendingQueue,
    pub outbound: PendingQueue,
    pub future_msgs: &'a mut Queue<Message>,

    /// State-related fields
    pub provisioners: &'a mut Provisioners,

    // Round/Step parameters
    pub round_update: RoundUpdate,
    pub step: u8,
}

impl<'a> ExecutionCtx<'a> {
    pub fn new(
        cancel_chan: &'a mut oneshot::Receiver<bool>,
        inbound: PendingQueue,
        outbound: PendingQueue,
        future_msgs: &'a mut Queue<Message>,
        provisioners: &'a mut Provisioners,
        round_update: RoundUpdate,
        step: u8,
    ) -> Self {
        Self {
            cancel_chan,
            inbound,
            outbound,
            future_msgs,
            provisioners,
            round_update,
            step,
        }
    }

    pub fn trace(&self, event_name: &'static str) {
        tracing::info!(
            "event={} round={}, step={}, bls_key={}",
            event_name,
            self.round_update.round,
            self.step,
            self.round_update.pubkey_bls.encode_short_hex()
        );
    }

    pub fn get_sortition_config(&self, size: usize) -> sortition::Config {
        sortition::Config::new(
            self.round_update.seed,
            self.round_update.round,
            self.step,
            size,
        )
    }

    pub fn handle_future_msgs<C: MsgHandler<Message>>(
        &self,
        committee: &Committee,
        phase: &mut C,
    ) -> Option<Message> {
        let ru = &self.round_update;

        if let Ok(messages) = self.future_msgs.get_events(ru.round, self.step) {
            for msg in messages {
                if let Ok(f) = phase.handle(msg, *ru, self.step, &committee) {
                    return Some(f.0);
                }
            }
        }

        None
    }

    // loop while waiting on multiple channels, a phase is interested in:
    // These are timeout, consensus_round and message channels.
    pub(crate) async fn event_loop<C: MsgHandler<Message>>(
        &mut self,
        committee: &Committee,
        phase: &mut C,
    ) -> Result<Message, SelectError> {
        let ru = self.round_update;
        let step = self.step;

        let deadline = Instant::now().checked_add(Duration::from_millis(5000));

        self.trace("run event_loop");

        loop {
            match select_multi(self.inbound.clone(), self.cancel_chan, deadline.unwrap()).await {
                // A message has arrived.
                // Delegate message processing and verification to the Step itself.
                // TODO: phase.is_valid { repropagate; handle }
                Ok(msg) => match phase.handle(msg.clone(), ru, self.step, committee) {
                    // Fully valid state reached  t5sxzxcZX on this step. Return it as an output.
                    // Populate next step with it.
                    Ok(result) => {
                        let msg = result.0;
                        self.outbound.send(msg.clone()).await;

                        if result.1 {
                            break Ok(msg);
                        }
                    }
                    // An error here means an invalid message has arrived.
                    // We need to continue waiting for either a valid message or timeout event.
                    Err(e) => {
                        match e {
                            ConsensusError::FutureEvent => {
                                // This is a message from future round or step. We store
                                // it in future_msgs to be process later on.
                                self.future_msgs.put_event(ru.round, step, msg);
                            }
                            ConsensusError::PastEvent => {
                                tracing::trace!("past event");
                            }
                            _ => {
                                warn!("error: {:?}", e);
                            }
                        }

                        continue;
                    }
                },
                // Error from select_multi means an loop-exit event.
                Err(e) => match e {
                    SelectError::Continue => continue,
                    SelectError::Timeout => {
                        // Timeout-ed step should proceed to next step with zero-ed.
                        break Ok(Message::empty());
                    }
                    SelectError::Canceled => {
                        break Err(e);
                    }
                },
            }
        }
    }
}

async fn select_multi(
    inbound: PendingQueue,
    cancel_chan: &mut oneshot::Receiver<bool>,
    deadline: time::Instant,
) -> Result<Message, SelectError> {
    tokio::select! {
        biased;
        // Handle both timeout and cancel events
        result = time::timeout_at(deadline, cancel_chan) => {
            match result {
            Ok(_) =>  Err(SelectError::Canceled),
            Err(_) => {
                 Err(SelectError::Timeout)
                }
            }
         },
        // Handle message
        msg = inbound.recv() => {
            match msg {
                Ok(m) => Ok(m),
                Err(_) => Err(SelectError::Continue),
            }
        },
    }
}
