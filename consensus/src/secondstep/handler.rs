use crate::aggregator::Aggregator;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, Hash, RoundUpdate};
use crate::event_loop::MsgHandler;

use crate::messages::{payload, Message, Payload};
use crate::user::committee::Committee;

pub struct Reduction {
    pub(crate) aggr: Aggregator,
}

impl MsgHandler<Message> for Reduction {
    // Collect the reduction message.
    fn handle_internal(
        &mut self,
        msg: Message,
        committee: &Committee,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Message, ConsensusError> {
        //TODO: VerifySignature
        //TODO: ??? Republish

        let msg_payload = match msg.payload {
            Payload::Reduction(p) => Ok(p),
            Payload::Empty => Ok(payload::Reduction::default()),
            _ => Err(ConsensusError::InvalidMsgType),
        };

        // Collect vote, if msg payload is of reduction type
        if let Some(sv) = self.aggr.collect_vote(committee, msg.header, msg_payload?) {
            // At that point, we have reached a quorum for 2th_reduction on an empty on non-empty block.
            self.send_agreement(ru, step, sv);
            // Return an empty message as this iteration terminates here.
            return Ok(Message::empty());
        }

        Err(ConsensusError::NotReady)
    }
}

impl Reduction {
    fn send_agreement(&self, _ru: RoundUpdate, _step: u8, _sv: (Hash, payload::StepVotes)) {
        // TODO: new agreement message

        // TODO: send agreement message to Agreement Loop channel
    }
}
