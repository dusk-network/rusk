// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};

use crate::consensus::Context;
use crate::event_loop::{event_loop, MsgHandler};
use crate::firststep::handler;
use crate::messages::Message;

use crate::frame;
use crate::frame::Frame;
use crate::queue::Queue;
use crate::user::committee::Committee;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    pub timeout: u16,
    handler: handler::Reduction,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
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
        };
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        inbound_msgs: &mut Receiver<Message>,
        outbound_msgs: &mut Sender<Message>,
        committee: Committee,
        future_msgs: &mut Queue<Message>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        if committee.am_member() {
            // TODO: SendReduction async
            // TODO: Register my reduction locally
        }

        // drain future messages for current round and step.
        if let Ok(messages) = future_msgs.get_events(ru.round, step).await {
            for msg in messages {
                if let Ok(f) = self.handler.handle(msg, ru, step, &committee) {
                    return Ok(f);
                }
            }
        }

        event_loop(
            &mut self.handler,
            ctx_recv,
            inbound_msgs,
            ru,
            step,
            &committee,
            future_msgs,
        )
        .await
    }

    pub fn name(&self) -> &'static str {
        "1th_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
