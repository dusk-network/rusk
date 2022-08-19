// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::event_loop;
use crate::messages::MsgReduction;
use crate::phase::*;
use crate::secondstep::handler;

use crate::frame::{Frame, StepVotes};
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tracing::trace;

#[allow(unused)]
pub struct Reduction {
    msg_rx: Receiver<MsgReduction>,

    handler: handler::Reduction,
}

impl Reduction {
    pub fn new(msg_rx: Receiver<MsgReduction>) -> Self {
        Self {
            msg_rx,
            handler: handler::Reduction {},
        }
    }

    pub fn initialize(&mut self, frame: &Frame) {
        let empty = StepVotes::default();

        let _step_votes = match frame {
            Frame::Empty => &empty,
            Frame::StepVotes(f) => f,
            Frame::NewBlock(_) => panic!("invalid frame"),
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

    pub fn name(&self) -> String {
        String::from("2nd_reduction")
    }
}

