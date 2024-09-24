// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::RoundUpdate;
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::proposal;
use crate::ratification::handler::RatificationHandler;
use crate::user::committee::Committee;
use crate::validation::handler::ValidationHandler;
use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::message::{Message, Payload, Status};
use node_data::StepName;
use tracing::{debug, warn};

/// Indicates whether an output value is available for current step execution
/// (Step is Ready) or needs to collect data (Step is Pending)
#[allow(clippy::large_enum_variant)]
pub enum HandleMsgOutput {
    Pending,
    Ready(Message),
}

/// MsgHandler must be implemented by any step that needs to handle an external
/// message within event_loop life-cycle.
#[async_trait]
pub trait MsgHandler {
    /// is_valid checks a new message is valid in the first place.
    ///
    /// Only if the message has correct round and step and is signed by a
    /// committee member then we delegate it to Phase::verify.
    fn is_valid(
        &self,
        msg: &Message,
        ru: &RoundUpdate,
        current_iteration: u8,
        step: StepName,
        committee: &Committee,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let signer = msg.get_signer().map(|s| s.to_bs58()).unwrap_or_default();

        debug!(
            event = "validating msg",
            signer,
            src_addr = ?msg.metadata.as_ref().map(|m| m.src_addr),
            topic = ?msg.topic(),
            step = msg.get_step(),
            ray_id = msg.ray_id(),
        );

        // We don't verify the tip here, otherwise future round messages will be
        // discarded and not put into the queue
        let msg_tip = msg.header.prev_block_hash;
        match msg.compare(ru.round, current_iteration, step) {
            Status::Past => {
                Self::verify_message(msg, ru, round_committees, Status::Past)?;
                Err(ConsensusError::PastEvent)
            }
            Status::Present => {
                if msg_tip != ru.hash() {
                    return Err(ConsensusError::InvalidPrevBlockHash(msg_tip));
                }
                let signer =
                    msg.get_signer().ok_or(ConsensusError::InvalidMsgType)?;

                // Ensure the message originates from a committee member.
                if !committee.is_member(&signer) {
                    return Err(ConsensusError::NotCommitteeMember);
                }

                // Delegate message final verification to the phase instance.
                // It is the phase that knows what message type to expect and if
                // it is valid or not.
                self.verify(msg, round_committees)
            }
            Status::Future => {
                Self::verify_message(
                    msg,
                    ru,
                    round_committees,
                    Status::Future,
                )?;
                Err(ConsensusError::FutureEvent)
            }
        }
    }

    /// Verify step message for the current round with different iteration
    fn verify_message(
        msg: &Message,
        ru: &RoundUpdate,
        round_committees: &RoundCommittees,
        status: Status,
    ) -> Result<(), ConsensusError> {
        // Pre-verify messages for the current round with different iteration
        if msg.header.round == ru.round {
            let msg_tip = msg.header.prev_block_hash;
            if msg_tip != ru.hash() {
                return Err(ConsensusError::InvalidPrevBlockHash(msg_tip));
            }

            let step = msg.get_step();
            if let Some(committee) = round_committees.get_committee(step) {
                if let Payload::ValidationQuorum(_) = &msg.payload {
                    // skip signer verification for ValidationQuorum, since
                    // there's no signer
                } else {
                    let signer = msg.get_signer().expect("signer to exist");
                    // Ensure the message originates from a committee
                    // member.
                    if !committee.is_member(&signer) {
                        return Err(ConsensusError::NotCommitteeMember);
                    }
                }

                match &msg.payload {
                    node_data::message::Payload::Ratification(_)
                    | node_data::message::Payload::ValidationQuorum(_) => {
                        RatificationHandler::verify_stateless(
                            msg,
                            round_committees,
                        )?;
                    }
                    node_data::message::Payload::Validation(_) => {
                        ValidationHandler::verify_stateless(
                            msg,
                            round_committees,
                        )?;
                    }
                    node_data::message::Payload::Candidate(c) => {
                        proposal::handler::verify_stateless(
                            c,
                            round_committees,
                        )?;
                    }
                    _ => {
                        warn!(
                            "{status:?} message not repropagated {:?}",
                            msg.topic()
                        );
                        Err(ConsensusError::InvalidMsgType)?;
                    }
                }
            } else {
                warn!("{status:?} committee for step {step} not generated; skipping pre-verification for {:?} message", msg.topic());
            }
        }
        Ok(())
    }

    /// verify allows each Phase to fully verify the message payload.
    fn verify(
        &self,
        msg: &Message,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError>;

    /// collect allows each Phase to process a verified inbound message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// collect allows each Phase to process a verified message from a former
    /// iteration
    async fn collect_from_past(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        committee: &Committee,
        generator: Option<PublicKeyBytes>,
    ) -> Result<HandleMsgOutput, ConsensusError>;

    /// handle_timeout allows each Phase to handle a timeout event.
    /// Returned Message here is sent to outboud queue.
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message>;
}
