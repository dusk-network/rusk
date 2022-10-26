// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{sign, verify_signature, ConsensusError, Hash, RoundUpdate, Topics};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use tracing::error;

use crate::aggregator::Aggregator;
use crate::messages;
use crate::messages::payload::StepVotes;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::Committee;

pub struct Reduction {
    pub(crate) aggr: Aggregator,
    pub(crate) first_step_votes: StepVotes,
}

impl MsgHandler<Message> for Reduction {
    fn verify(
        &mut self,
        msg: Message,
        _ru: RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        let msg_payload = match msg.payload {
            Payload::Reduction(p) => Ok(p),
            Payload::Empty => Ok(payload::Reduction::default()),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if let Err(e) = verify_signature(&msg.header, msg_payload.signed_hash) {
            error!("verify_signature err: {}", e);
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(msg)
    }

    /// Collect the reduction message.
    fn collect(
        &mut self,
        msg: Message,
        ru: RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let msg_payload = match msg.payload {
            Payload::Reduction(p) => Ok(p),
            Payload::Empty => Ok(payload::Reduction::default()),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is of reduction type
        if let Some(sv) = self.aggr.collect_vote(committee, msg.header, msg_payload) {
            // At that point, we have reached a quorum for 2th_reduction on an empty on non-empty block.
            // Return an empty message as this iteration terminates here.

            return Ok(HandleMsgOutput::FinalResult(
                self.build_agreement_msg(ru, step, sv),
            ));
        }

        Ok(HandleMsgOutput::Result(msg))
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::FinalResult(Message::empty()))
    }
}

impl Reduction {
    fn build_agreement_msg(
        &self,
        ru: RoundUpdate,
        step: u8,
        sv: (Hash, payload::StepVotes),
    ) -> Message {
        let hdr = messages::Header {
            pubkey_bls: ru.pubkey_bls,
            round: ru.round,
            step,
            block_hash: sv.0,
            topic: Topics::Agreement as u8,
        };

        let payload = payload::Agreement {
            signature: sign(ru.secret_key, ru.pubkey_bls.to_bls_pk(), hdr),
            votes_per_step: (self.first_step_votes, sv.1),
        };

        Message::new_agreement(hdr, payload)
    }
}
