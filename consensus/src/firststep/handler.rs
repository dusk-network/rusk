use crate::aggregator::Aggregator;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, RoundUpdate};
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
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<Message, ConsensusError> {
        //TODO: VerifySignature
        //TODO: if passes, Republish

        let msg_payload = match msg.payload {
            Payload::Reduction(p) => Ok(p),
            Payload::Empty => Ok(payload::Reduction::default()),
            _ => Err(ConsensusError::InvalidMsgType),
        };

        // Collect vote, if msg payload is reduction type
        if let Some(sv) = self.aggr.collect_vote(committee, msg.header, msg_payload?) {
            // At that point, we have reached a quorum for 1th_reduction on an empty on non-empty block
            return Ok(Message::from_stepvotes(sv.1));
        }

        Err(ConsensusError::NotReady)
    }
}
