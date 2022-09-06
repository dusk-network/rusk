// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, RoundUpdate};
use crate::event_loop::MsgHandler;
use crate::messages::Message;

pub struct Reduction {}

impl MsgHandler<Message> for Reduction {
    // Collect the reduction message.
    fn handle_internal(
        &mut self,
        _msg: Message,
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<Message, ConsensusError> {
        //TODO: VerifySignature
        //TODO: Republish
        //TODO: CollectVote
        self.collect_vote();

        Err(ConsensusError::NotImplemented)
    }
}

impl Reduction {
    fn collect_vote(&mut self) {
        //TODO Collect vote
    }
}
