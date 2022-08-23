// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::frame::Frame;
use crate::messages::Message;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use tokio::{select, time};
use tracing::trace;

// loop while waiting on multiple channels, a phase is interested in:
// These are timeout, consensus_round and message channels.
pub async fn event_loop<T: Default + Debug + Message, C: MsgHandler<T>>(
    phase: &mut C,
    rx: &mut mpsc::Receiver<T>,
    ctx_recv: &mut oneshot::Receiver<Context>,
    ru: RoundUpdate,
    step: u8,
) -> Result<Frame, SelectError> {
    let deadline = Instant::now().checked_add(Duration::from_millis(5000));

    loop {
        match select_multi(rx, ctx_recv, deadline.unwrap()).await {
            // A message has arrived.
            // Delegate message processing and verification to the Step itself.
            Ok(msg) => match phase.handle(msg, ru, step) {
                // Fully valid state reached on this step. Return it as an output.
                // Populate next step with it.
                Ok(f) => break Ok(f),
                // An error here means an invalid message has arrived.
                // We need to continue waiting for either a valid message or timeout event.
                Err(_e) => continue,
            },
            // Error from select_multi means an loop-exit event.
            Err(e) => match e {
                SelectError::Continue => continue,
                SelectError::Timeout => {
                    // Timeout-ed step should proceed to next step with zero-ed.
                    break Ok(Frame::Empty);
                }
                SelectError::Canceled => {
                    break Err(e);
                }
            },
        }
    }
}

// MsgHandler must be implemented by any step that needs to handle an external message within event_loop life-cycle.
pub trait MsgHandler<T: Debug + Message> {
    // handle is the handler to process a new message in the first place.
    // Only if it's valid to current round and step, it delegates it to the Phase::handler.
    fn handle(&mut self, msg: T, ru: RoundUpdate, step: u8) -> Result<Frame, ConsensusError> {
        trace!("handle msg {:?}", msg);

        // this the equivalent of should_process.
        if !msg.compare(ru.round, step) {
            return Err(ConsensusError::InvalidRoundStep);
        }

        self.handle_internal(msg, ru, step)
    }

    // handle_internal should be implemented by each Phase.
    fn handle_internal(
        &mut self,
        msg: T,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, ConsensusError>;
}

// select_multi extends time::timeout_at with another channel that brings the message payload.
async fn select_multi<T: Default>(
    msg_recv: &mut mpsc::Receiver<T>,
    ctx_recv: &mut oneshot::Receiver<Context>,
    deadline: time::Instant,
) -> Result<T, SelectError> {
    select! {
        // Handle message
        val = (*msg_recv).recv() => {
            match val {
                Some(res) => Ok(res),
                None => Err(SelectError::Continue),
            }
        },
        // Handle both timeout and cancel events
        result = time::timeout_at(deadline, ctx_recv) => {
            match result {
            Ok(_) =>  Err(SelectError::Canceled),
            Err(_) => {
                 Err(SelectError::Timeout)
                }
            }
         }
    }
}
