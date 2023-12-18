// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::execution_ctx::RoundCommittees;
use crate::user::committee::Committee;
use async_trait::async_trait;
use node_data::ledger::to_str;
use node_data::message::{Message, MessageTrait, Status, Topics};
use std::fmt::Debug;
use tracing::{debug, trace};

/// Indicates whether an output value is available for current step execution
/// (Step is Ready) or needs to collect data (Step is Pending)
pub enum HandleMsgOutput {
    Pending(Message),
    Ready(Message),
    ReadyWithTimeoutIncrease(Message),
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
        round_committees: &RoundCommittees,
    ) -> Result<T, ConsensusError> {
        debug!(
            event = "msg received",
            from = msg.get_pubkey_bls().to_bs58(),
            hash = to_str(&msg.get_block_hash()),
            topic = format!("{:?}", Topics::from(msg.get_topic())),
            step = msg.get_step(),
        );

        trace!(event = "msg received", msg = format!("{:#?}", msg),);

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
                self.verify(msg, ru, step, committee, round_committees)
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
        round_committees: &RoundCommittees,
    ) -> Result<T, ConsensusError>;

    /// collect allows each Phase to process a verified inbound message.
    async fn collect(
        &mut self,
        msg: T,
        ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// collect allows each Phase to process a verified message from a former
    /// iteration
    async fn collect_from_past(
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
