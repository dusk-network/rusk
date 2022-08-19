use tracing::trace;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, RoundUpdate};
use crate::event_loop::MsgHandler;
use crate::frame::Frame;
use crate::messages::MsgNewBlock;
pub struct Selection {}

impl MsgHandler<MsgNewBlock> for Selection {
    // Handle а new_block message.
    fn handle(
        &mut self,
        msg: MsgNewBlock,
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<Frame, ConsensusError> {
        //TODO: should_process
        trace!("handle msg {:?}", msg);

        match self.verify(&msg) {
            None => self.on_valid_new_block(&msg),
            Some(_err) => Err(ConsensusError::InvalidBlock),
        }
    }
}

impl Selection {
    fn verify(&self, _msg: &MsgNewBlock) -> Option<ConsensusError> {
        // TODO: Verify newblock msg signature
        // TODO: Verify newblock candidate
        Some(ConsensusError::NotImplemented)
    }

    fn on_valid_new_block(&mut self, _msg: &MsgNewBlock) -> Result<Frame, ConsensusError> {
        // TODO: store candidate block
        // TODO: republish new_block
        Ok(Frame::Nil)
    }
}
