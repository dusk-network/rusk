// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::phase::*;

use crate::consensus::Context;
use crate::event_loop::event_loop;
use crate::firststep::handler;
use crate::messages::MsgReduction;

use crate::frame;
use crate::frame::Frame;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tracing::trace;

#[allow(unused)]
pub struct Reduction {
    msg_rx: Receiver<MsgReduction>,

    pub timeout: u16,
    handler: handler::Reduction,
}

impl Reduction {
    pub fn new(msg_rx: Receiver<MsgReduction>) -> Self {
        Self {
            msg_rx,
            timeout: 0,
            handler: handler::Reduction {},
        }
    }

    pub fn initialize(&mut self, frame: &Frame) {
        let empty = frame::NewBlock::default();

        let mut _new_block = match frame {
            Frame::NewBlock(f) => f,
            Frame::StepVotes(_) => panic!("invalid frame"),
            Frame::Empty => &empty,
            Frame::Nil => &empty,
        };

        trace!("initializing with frame: {:?}  ", frame);
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        // TODO: If isMember()
        // TODO: send_reduction in async way

        trace!("running {:?} round:{} step:{}", self.name(), ru.round, step);

        event_loop(&mut self.handler, &mut self.msg_rx, ctx_recv, ru, step).await
    }

    fn name(&self) -> String {
        String::from("1th_reduction")
    }
}
