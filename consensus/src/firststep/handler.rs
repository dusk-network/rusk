// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, RoundUpdate};
use node_common::ledger;
use node_common::ledger::{Block, Signature, StepVotes};

use crate::msg_handler::{HandleMsgOutput, MsgHandler};

use crate::aggregator::Aggregator;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::Committee;

macro_rules! empty_result {
    (  ) => {
        HandleMsgOutput::FinalResult(Message::from_stepvotes(
            payload::StepVotesWithCandidate {
                sv: StepVotes::default(),
                candidate: Block::default(),
            },
        ))
    };
}

#[derive(Default)]
pub struct Reduction {
    pub(crate) aggr: Aggregator,
    pub(crate) candidate: Block,
}

impl MsgHandler<Message> for Reduction {
    /// Verifies if a msg is a valid reduction message.
    fn verify(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signed_hash),
            Payload::Empty => Ok(Signature::default().0),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if msg.header.verify_signature(&signed_hash).is_err() {
            return Err(ConsensusError::InvalidSignature);
        }

        Ok(msg)
    }

    /// Collects the reduction message.
    fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _step: u8,
        committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        let signed_hash = match &msg.payload {
            Payload::Reduction(p) => Ok(p.signed_hash),
            Payload::Empty => Ok(Signature::default().0),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        // Collect vote, if msg payload is reduction type
        if let Some((hash, sv)) =
            self.aggr.collect_vote(committee, &msg.header, &signed_hash)
        {
            // if the votes converged for an empty hash we invoke halt
            if hash == [0u8; 32] {
                tracing::warn!("votes converged for an empty hash");
                // TODO: increase timeout
                return Ok(empty_result!());
            }

            if hash != self.candidate.header.hash {
                tracing::warn!("request candidate block from peers");
                // TODO: Fetch Candidate procedure
                return Ok(empty_result!());
            }

            // At that point, we have reached a quorum for 1th_reduction on an empty on non-empty block
            return Ok(HandleMsgOutput::FinalResult(Message::from_stepvotes(
                payload::StepVotesWithCandidate {
                    sv,
                    candidate: self.candidate.clone(),
                },
            )));
        }

        Ok(HandleMsgOutput::Result(msg))
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &mut self,
        _ru: &RoundUpdate,
        _step: u8,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        Ok(HandleMsgOutput::FinalResult(Message::from_stepvotes(
            payload::StepVotesWithCandidate {
                sv: ledger::StepVotes::default(),
                candidate: self.candidate.clone(),
            },
        )))
    }
}
