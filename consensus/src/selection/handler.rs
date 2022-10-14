// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{verify_signature, ConsensusError, RoundUpdate};
use crate::messages::{Message, Payload};
use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::user::committee::Committee;

pub struct Selection {}

impl MsgHandler<Message> for Selection {
    fn verify(
        &mut self,
        msg: Message,
        _ru: RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<Message, ConsensusError> {
        self.verify_new_block(&msg)?;

        Ok(msg)
    }

    /// collect а new_block message.
    fn collect(
        &mut self,
        msg: Message,
        _ru: RoundUpdate,
        _step: u8,
        _committee: &Committee,
    ) -> Result<HandleMsgOutput, ConsensusError> {
        // TODO: store candidate block

        Ok(HandleMsgOutput::FinalResult(msg))
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

impl Selection {
    fn verify_new_block(&self, msg: &Message) -> Result<(), ConsensusError> {
        //  Verify new_block msg signature
        if let Payload::NewBlock(p) = msg.clone().payload {
            if verify_signature(&msg.header, p.signed_hash).is_err() {
                return Err(ConsensusError::InvalidSignature);
            }

            // TODO: Verify newblock candidate
            return Ok(());
        }

        Err(ConsensusError::InvalidMsgType)
    }
}
