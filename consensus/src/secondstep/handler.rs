// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use async_trait::async_trait;
use node_data::ledger;
use node_data::ledger::{Hash, Signature, StepVotes};
use tracing::error;

use crate::aggregator::Aggregator;
use node_data::message::{payload, Message, Payload, Topics};

use crate::user::committee::Committee;

pub struct Reduction {
    pub(crate) aggr: Aggregator,
    pub(crate) first_step_votes: StepVotes,
}

#[async_trait]
impl MsgHandler<Message> for Reduction {
    fn verify(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signature),
            Payload::Empty => Ok(Signature::default().0),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if let Err(e) = msg.header.verify_signature(&signed_hash) {
            error!("verify_signature err: {}", e);
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(msg)
    }

    /// Collect the reduction message.
    async fn collect(
        &mut self,
        msg: Message,
        ru: &RoundUpdate,
        step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signature),
            Payload::Empty => Ok(Signature::default().0),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is of reduction type
        if let Some((block_hash, second_step_votes)) =
            self.aggr.collect_vote(committee, &msg.header, &signed_hash)
        {
            // At that point, we have reached a quorum for 2th_reduction on an
            // empty on non-empty block. Return an empty message as
            // this iteration terminates here.
            return Ok(HandleMsgOutput::FinalResult(self.build_agreement_msg(
                ru,
                step,
                block_hash,
                second_step_votes,
            )));
        }

        Ok(HandleMsgOutput::Result(msg))
    }

    /// Handle of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::FinalResult(Message::empty()))
    }
}

impl Reduction {
    fn build_agreement_msg(
        &self,
        ru: &RoundUpdate,
        step: u8,
        block_hash: Hash,
        second_step_votes: ledger::StepVotes,
    ) -> Message {
        let hdr = node_data::message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            step,
            block_hash,
            topic: Topics::Agreement as u8,
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());
        let payload = payload::Agreement {
            signature,
            first_step: self.first_step_votes.clone(),
            second_step: second_step_votes,
        };

        Message::new_agreement(hdr, payload)
    }

    pub(crate) fn reset(&mut self) {
        self.aggr = Aggregator::default();
        self.first_step_votes = StepVotes::default();
    }
}
