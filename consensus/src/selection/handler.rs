// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, RoundUpdate};
use crate::event_loop::MsgHandler;
use crate::messages::Message;
use crate::user::committee::Committee;

pub struct Selection {}

impl MsgHandler<Message> for Selection {
    // Handle а new_block message.
    fn handle_internal(
        &mut self,
        msg: Message,
        _committee: &Committee,
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<(Message, bool), ConsensusError> {
        match self.verify(&msg) {
            Ok(_) => self.on_valid_new_block(&msg),
            Err(err) => Err(err),
        }
    }
}

impl Selection {
    fn verify(&self, _msg: &Message) -> Result<(), ConsensusError> {
        // TODO: Verify newblock msg signature
        // TODO: Verify newblock candidate
        Ok(())
    }

    fn on_valid_new_block(&mut self, msg: &Message) -> Result<(Message, bool), ConsensusError> {
        // TODO: store candidate block
        Ok((msg.clone(), true))
    }
}
