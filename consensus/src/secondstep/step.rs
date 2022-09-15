// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::event_loop;
use crate::event_loop::MsgHandler;
use crate::messages::Message;
use crate::secondstep::handler;
use crate::user::committee::Committee;
use crate::user::provisioners::PublicKey;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use crate::queue::Queue;
use tracing::debug;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    handler: handler::Reduction,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
            handler: handler::Reduction {
                aggr: Default::default(),
            },
        }
    }

    pub fn initialize(&mut self, _msg: &Message) {
        /*
        let empty = StepVotes::default();

        let _step_votes = match msg.payload {
            payload::NewBlock => panic!("invalid frame"),
            Frame::StepVotes(f) => f,
            Frame::NewBlock(_) => panic!("invalid frame"),
        };

         */
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        inbound_msgs: &mut Receiver<Message>,
        _outbound_msgs: &mut Sender<Message>,
        committee: Committee,
        future_msgs: &mut Queue<Message>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Message, SelectError> {
        if committee.am_member() {
            self.spawn_send_reduction(committee.get_my_pubkey(), ru.round, step);
            // TODO: Register my reduction locally
        }

        // drain future queued messages
        if let Ok(messages) = future_msgs.get_events(ru.round, step) {
            for msg in messages {
                if let Ok(f) = self.handler.handle(msg, ru, step, &committee) {
                    return Ok(f);
                }
            }
        }

        match event_loop(
            &mut self.handler,
            ctx_recv,
            inbound_msgs,
            ru,
            step,
            &committee,
            future_msgs,
        )
        .await
        {
            Err(SelectError::Timeout) => {
                //TODO create agreement with empty block
                // self.handler.on_timeout();
                Ok(Message::empty())
            }
            Err(err) => Err(err),
            Ok(res) => Ok(res),
        }
    }

    pub fn name(&self) -> &'static str {
        "2nd_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }

    fn spawn_send_reduction(&self, pubkey: PublicKey, round: u64, step: u8) {
        let name = self.name();
        tokio::spawn(async move {
            debug!(
                "send reduction at {} round={}, step={}, bls_key={}",
                name,
                round,
                step,
                pubkey.encode_short_hex()
            );
        });
    }
}
