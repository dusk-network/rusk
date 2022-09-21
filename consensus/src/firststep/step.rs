use bytes::{Buf, BufMut, BytesMut};
use dusk_bls12_381_sign::{SecretKey, APK};
use dusk_bytes::Serializable;
use std::ops::Deref;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::aggregator::Aggregator;
use crate::commons::{sign, Block, RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::{event_loop, MsgHandler};
use crate::firststep::handler;
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::util::pubkey::PublicKey;
use hex::ToHex;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tracing::info;

pub const COMMITTEE_SIZE: usize = 64;

#[allow(unused)]
pub struct Reduction {
    pub timeout: u16,
    handler: handler::Reduction,
    selection_result: Box<payload::NewBlock>,
}

impl Reduction {
    pub fn new() -> Self {
        Self {
            timeout: 0,
            handler: handler::Reduction {
                aggr: Aggregator::default(),
                candidate: Block::default(),
            },
            selection_result: Default::default(),
        }
    }

    pub fn initialize(&mut self, msg: &Message) {
        // TODO move msg here instead of clone
        self.selection_result = Box::new(payload::NewBlock::default());

        if let Payload::NewBlock(p) = msg.clone().payload {
            self.selection_result = p.clone();

            // TODO: that's ugly
            self.handler.candidate = p.deref().candidate.clone();
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
            //  : Send reduction async
            self.spawn_send_reduction(committee.get_my_pubkey(), ru, step, outbound_msgs.clone());

            // TODO: Register my reduction locally
        }

        // drain future messages for current round and step.
        if let Ok(messages) = future_msgs.get_events(ru.round, step) {
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

    fn spawn_send_reduction(
        &self,
        pubkey: PublicKey,
        ru: RoundUpdate,
        step: u8,
        outbound: Sender<Message>,
    ) {
        let name = self.name();
        let selection_result = self.selection_result.clone().deref().clone();

        tokio::spawn(async move {
            // TODO: use info_span
            info!(
                "send reduction at {} round={}, step={}, bls_key={} hash={}",
                name,
                ru.round,
                step,
                pubkey.encode_short_hex(),
                selection_result
                    .candidate
                    .header
                    .hash
                    .as_slice()
                    .encode_hex::<String>(),
            );

            // TODO: VerifyStateTransition call here

            let mut hdr = messages::Header {
                pubkey_bls: pubkey,
                round: ru.round,
                step,
                block_hash: selection_result.candidate.header.hash,
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
        });
    }

    pub fn name(&self) -> &'static str {
        "1th_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
