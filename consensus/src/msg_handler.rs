// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate, SelectError};
use crate::execution_ctx::ExecutionCtx;
use crate::messages::{Message, MessageTrait, Status};
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::util::pending_queue::PendingQueue;
use hex::ToHex;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use tokio::{select, time};
use tracing::{info, warn};

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
    ) -> Result<(Message, bool), ConsensusError> {
        tracing::trace!(
            "received msg from {:?} with hash {} msg: {:?}",
            msg.get_pubkey_bls().encode_short_hex(),
            msg.get_block_hash().encode_hex::<String>(),
            msg,
        );

        match msg.compare(ru.round, step) {
            Status::Past => Err(ConsensusError::PastEvent),
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
    ) -> Result<(Message, bool), ConsensusError>;
}
