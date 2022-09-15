// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::messages::{Message, MessageTrait, Status};
use crate::queue::Queue;
use crate::user::committee::Committee;
use hex::ToHex;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use tokio::{select, time};
use tracing::{debug, info};

// loop while waiting on multiple channels, a phase is interested in:
// These are timeout, consensus_round and message channels.
pub async fn event_loop<C: MsgHandler<Message>>(
    phase: &mut C,
    ctx_recv: &mut oneshot::Receiver<Context>,
    inbound_msgs: &mut mpsc::Receiver<Message>,
    ru: RoundUpdate,
    step: u8,
    committee: &Committee,
    future_msgs: &mut Queue<Message>,
) -> Result<Message, SelectError> {
    let deadline = Instant::now().checked_add(Duration::from_millis(5000));

    // TODO: Since introducing inbound_msgs_queue, the tokio::runtime does not react accurately on timeout_at(deadline)
    /*
        [2mSep 02 14:07:06.00[0m[32m INF[0m consensus::event_loop: 2286d081884c7d65 start event_loop round: 0, step: 1 deadline: Some(Instant { tv_sec: 14832, tv_nsec: 797601650 })
        [2mSep 02 14:07:23.01[0m[32m INF[0m consensus::event_loop: 2286d081884c7d65 end event_loop round: 0, step: 1
    */

    info!(
        "{} start event_loop round: {}, step: {} deadline: {:?}",
        ru.pubkey_bls.encode_short_hex(),
        ru.round,
        step,
        deadline
    );

    let res = loop {
        match select_multi(inbound_msgs, ctx_recv, deadline.unwrap()).await {
            // A message has arrived.
            // Delegate message processing and verification to the Step itself.
            Ok(msg) => match phase.handle(msg.clone(), ru, step, committee) {
                // Fully valid state reached on this step. Return it as an output.
                // Populate next step with it.
                Ok(f) => break Ok(f),
                // An error here means an invalid message has arrived.
                // We need to continue waiting for either a valid message or timeout event.
                Err(e) => {
                    if e == ConsensusError::FutureEvent {
                        // This is a message from future round or step. We store
                        // it in future_msgs to be process later on.
                        future_msgs.put_event(ru.round, step, msg);
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
    };

    info!(
        "{} end event_loop round: {}, step: {}",
        ru.pubkey_bls.encode_short_hex(),
        ru.round,
        step
    );
    res
}

// MsgHandler must be implemented by any step that needs to handle an external message within event_loop life-cycle.
pub trait MsgHandler<T: Debug + MessageTrait> {
    // handle is the handler to process a new message in the first place.
    // Only if it's valid to current round and step, it delegates it to the Phase::handler.
    fn handle(
        &mut self,
        msg: T,
        ru: RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        debug!(
            "received msg from {:?} with hash {}",
            msg.get_pubkey_bls().encode_short_hex(),
            msg.get_block_hash().encode_hex::<String>(),
        );

        match msg.compare(ru.round, step) {
            Status::Past => Err(ConsensusError::InvalidRoundStep),
            Status::Present => {
                // Ensure the message originates from a committee member.
                if !committee.is_member(msg.get_pubkey_bls()) {
                    return Err(ConsensusError::NotCommitteeMember);
                }

                // Delegate message handling to the phase implementation.
                self.handle_internal(msg, committee, ru, step)
            }
            Status::Future => Err(ConsensusError::FutureEvent),
        }
    }

    // handle_internal should be implemented by each Phase.
    fn handle_internal(
        &mut self,
        msg: T,
        committee: &Committee,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Message, ConsensusError>;
}

// select_simple wraps up time::timeout_at with need of ctx_recv.
#[allow(unused)]
async fn select_simple<T: Default>(
    inbound_msgs: &mut mpsc::Receiver<T>,
    ctx_recv: &mut oneshot::Receiver<Context>,
    deadline: time::Instant,
) -> Result<T, SelectError> {
    // Handle both timeout and inbound events
    if let Ok(val) = time::timeout_at(deadline, (*inbound_msgs).recv()).await {
        match val {
            Some(res) => Ok(res),
            None => Err(SelectError::Continue),
        }
    } else {
        Err(SelectError::Timeout)
    }
}

async fn select_multi<T: Default>(
    inbound_msgs: &mut mpsc::Receiver<T>,
    ctx_recv: &mut oneshot::Receiver<Context>,
    deadline: time::Instant,
) -> Result<T, SelectError> {
    select! {
        biased;
        // Handle both timeout and cancel events
        result = time::timeout_at(deadline, ctx_recv) => {
            match result {
            Ok(_) =>  Err(SelectError::Canceled),
            Err(_) => {
                 Err(SelectError::Timeout)
                }
            }
         },
        // Handle message
        val = (*inbound_msgs).recv() => {
            match val {
                Some(res) => Ok(res),
                None => Err(SelectError::Continue),
            }
        },
    }
}
