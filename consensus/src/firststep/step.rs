use std::ops::Deref;
use std::time::Duration;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::aggregator::Aggregator;
use crate::commons::{sign, Block, RoundUpdate, SelectError};

use crate::execution_ctx::ExecutionCtx;
use crate::firststep::handler;
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::msg_handler::MsgHandler;
use crate::queue::Queue;
use crate::user::committee::Committee;
use crate::util::pending_queue::PendingQueue;
use crate::util::pubkey::PublicKey;
use hex::ToHex;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::sleep;
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
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, SelectError> {
        let ru = ctx.round_update;
        let step = ctx.step;

        if committee.am_member() {
            //  Send reduction async
            self.spawn_send_reduction(
                committee.get_my_pubkey(),
                ru,
                step,
                ctx.outbound.clone(),
                ctx.inbound.clone(),
            );
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler) {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler).await
    }

    fn spawn_send_reduction(
        &self,
        pubkey: PublicKey,
        ru: RoundUpdate,
        step: u8,
        mut outbound: PendingQueue,
        mut inbound: PendingQueue,
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
            // Simulate VerifyStateTransition execution time
            // tokio::time::sleep(Duration::from_secs(3)).await;

            let hdr = messages::Header {
                pubkey_bls: pubkey,
                round: ru.round,
                step,
                block_hash: selection_result.candidate.header.hash,
            };

            let msg = Message::new_reduction(
                hdr,
                messages::payload::Reduction {
                    signed_hash: sign(ru.secret_key, ru.pubkey_bls.to_bls_pk(), hdr),
                },
            );

            // sign and publish
            outbound.send(msg.clone()).await;

            // Register my vote locally
            inbound.send(msg).await;
        });
    }

    pub fn name(&self) -> &'static str {
        "1th_reduction"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}
