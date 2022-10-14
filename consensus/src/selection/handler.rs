// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{verify_signature, ConsensusError, RoundUpdate};
use crate::messages::{payload, Message, Payload};
use crate::msg_handler::MsgHandler;
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
        let _new_block = self.verify(&msg)?;

        // TODO: store candidate block

        Ok((msg, true))
    }
}

impl Selection {
    fn verify(&self, msg: &Message) -> Result<payload::NewBlock, ConsensusError> {
        //  Verify new_block msg signature
        if let Payload::NewBlock(p) = msg.clone().payload {
            if verify_signature(&msg.header, p.signed_hash).is_err() {
                return Err(ConsensusError::InvalidSignature);
            }

            // TODO: Verify newblock candidate

            return Ok(*p);
        }

        Err(ConsensusError::InvalidMsgType)
    }
}
