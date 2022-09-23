// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{sign, RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::event_loop;
use crate::event_loop::MsgHandler;
use crate::messages::{payload, Message, Payload};
use crate::secondstep::handler;
use crate::user::committee::Committee;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;

use crate::messages;
use crate::queue::Queue;
use crate::util::pubkey::PublicKey;
use tracing::{debug, info};

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    handler: handler::Reduction,
    msg: Message,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
            handler: handler::Reduction {
                aggr: Default::default(),
                first_step_votes: payload::StepVotes {
                    bitset: 0,
                    signature: [0; 48],
                },
            },
            msg: Message::empty(),
        }
    }

    pub fn initialize(&mut self, msg: &Message) {
        self.msg = msg.clone();

        if let Payload::StepVotesWithCandidate(p) = msg.payload.clone() {
            self.handler.first_step_votes = p.sv;
        }
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
    ) -> Result<Message, SelectError> {
        if committee.am_member() {
            self.spawn_send_reduction(committee.get_my_pubkey(), ru, step, outbound_msgs.clone());
            // TODO: Register my reduction locally
        }

        // drain future queued messages
        if let Ok(messages) = future_msgs.get_events(ru.round, step) {
            for msg in messages {
                if let Ok(f) = self.handler.handle(msg, ru, step, &committee) {
                    return Ok(f.0);
                }
            }
        }

        match event_loop(
            &mut self.handler,
            ctx_recv,
            inbound_msgs,
            outbound_msgs.clone(),
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

    fn spawn_send_reduction(
        &self,
        pubkey: PublicKey,
        ru: RoundUpdate,
        step: u8,
        outbound: Sender<Message>,
    ) {
        use hex::ToHex;

        let name = self.name();
        let msg = self.msg.clone();
        tokio::spawn(async move {
            if let Payload::StepVotesWithCandidate(p) = msg.payload {
                info!(
                    "send 2th reduction at {} round={}, step={}, bls_key={} hash={}",
                    name,
                    ru.round,
                    step,
                    pubkey.encode_short_hex(),
                    p.candidate.header.hash.as_slice().encode_hex::<String>(),
                );

                let hdr = messages::Header {
                    pubkey_bls: pubkey,
                    round: ru.round,
                    step,
                    block_hash: p.candidate.header.hash,
                };

                // sign and publish
                outbound
                    .send(Message::new_reduction(
                        hdr,
                        messages::payload::Reduction {
                            signed_hash: sign(ru.secret_key, ru.pubkey_bls.to_bls_pk(), hdr),
                        },
                    ))
                    .await;
            }
        });
    }
}
