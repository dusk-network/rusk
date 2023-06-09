// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::user::committee::Committee;
use async_trait::async_trait;
use hex::ToHex;
use node_data::message::{Message, MessageTrait, Status};
use std::fmt::Debug;

pub enum HandleMsgOutput {
    Result(Message),
    FinalResult(Message),
}

/// MsgHandler must be implemented by any step that needs to handle an external
/// message within event_loop life-cycle.
#[async_trait]
pub trait MsgHandler<T: Debug + MessageTrait> {
    /// is_valid checks a new message is valid in the first place.
    ///
    /// Only if the message has correct round and step and is signed by a
    /// committee member then we delegate it to Phase::verify.
    fn is_valid(
        &mut self,
        msg: T,
        ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<T, ConsensusError> {
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

                // Delegate message final verification to the phase instance.
                // It is the phase that knows what message type to expect and if
                // it is valid or not.
                self.verify(msg, ru, step, committee)
            }
            Status::Future => Err(ConsensusError::FutureEvent),
        }
    }

    /// verify allows each Phase to fully verify the message payload.
    fn verify(
        &mut self,
        msg: T,
        ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<T, ConsensusError>;

    /// collect allows each Phase to process a verified inbound message.
    async fn collect(
        &mut self,
        msg: T,
        ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// handle_timeout allows each Phase to handle a timeout event.
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError>;
}
