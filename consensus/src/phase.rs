// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;

use crate::frame::Frame;
use tokio::sync::oneshot;
use crate::selection;
use crate::{firststep, secondstep};

pub enum Phase{
    Selection(selection::step::Selection),
    Reduction1(firststep::step::Reduction),
    Reduction2(secondstep::step::Reduction),
}

impl Phase {
    pub fn initialize(&mut self, frame: &Frame) {
        match self {
            Self::Selection(sel) => sel.initialize(frame),
            Self::Reduction1(red_1) => red_1.initialize(frame),
            Self::Reduction2(red_2) => red_2.initialize(frame),
        };
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        match self {
            Self::Selection(sel) => sel.run(ctx_recv, ru, step).await,
            Self::Reduction1(red_1) => red_1.run(ctx_recv, ru, step).await,
            Self::Reduction2(red_2) => red_2.run(ctx_recv, ru, step).await,
        }
    }

}