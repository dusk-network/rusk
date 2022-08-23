// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{ConsensusError, RoundUpdate};
use crate::event_loop::MsgHandler;
use crate::frame::Frame;
use crate::messages::MsgReduction;
pub struct Reduction {}

impl MsgHandler<MsgReduction> for Reduction {
    // Collect the reduction message.
    fn handle_internal(
        &mut self,
        _msg: MsgReduction,
        _ru: RoundUpdate,
        _step: u8,
    ) -> Result<Frame, ConsensusError> {
        //TODO: IsMember

        //TODO: VerifySignature
        //TODO: ??? Republish
        //TODO: CollectVote

        Err(ConsensusError::NotImplemented)
    }
}
