use hex::ToHex;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons;
use crate::commons::{Block, RoundUpdate, SelectError};
use crate::execution_ctx::ExecutionCtx;
use crate::messages::{payload::NewBlock, Header, Message};
use crate::msg_handler::MsgHandler;

use crate::selection::handler;
use crate::user::committee::Committee;
use crate::util::pubkey::PublicKey;
use sha3::{Digest, Sha3_256};
use tracing::{error, info};

pub const COMMITTEE_SIZE: usize = 1;

pub struct Selection {
    handler: handler::Selection,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            handler: handler::Selection {},
        }
    }

    pub fn initialize(&mut self, _msg: &Message) {}

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_>,
        committee: Committee,
    ) -> Result<Message, SelectError> {
        if committee.am_member() {
            let msg =
                self.generate_candidate(committee.get_my_pubkey(), ctx.round_update, ctx.step);

            // Broadcast the candidate block for this round/iteration.
            if let Err(e) = ctx.outbound.send(msg.clone()).await {
                error!("could not send newblock msg due to {:?}", e);
            }

            // register new candidate in local state
            match self
                .handler
                .handle(msg, ctx.round_update, ctx.step, &committee)
            {
                Ok(f) => return Ok(f.0),
                Err(e) => error!("invalid candidate generated due to {:?}", e),
            };
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(&committee, &mut self.handler) {
            return Ok(m);
        }

        ctx.event_loop(&committee, &mut self.handler).await
    }

    pub fn name(&self) -> &'static str {
        "selection"
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}

impl Selection {
    // generate_candidate generates a hash to propose.
    fn generate_candidate(&self, pubkey: PublicKey, ru: RoundUpdate, step: u8) -> Message {
        let mut hasher = Sha3_256::new();
        hasher.update(ru.round.to_le_bytes());
        hasher.update(step.to_le_bytes());
        hasher.update(123_i32.to_le_bytes());

        let hash = hasher.finalize();

        info!(
            "generate candidate block hash={} round={}, step={}, bls_key={}",
            hash.as_slice().encode_hex::<String>(),
            ru.round,
            step,
            pubkey.encode_short_hex()
        );

        // TODO: refactor this
        let a = NewBlock {
            prev_hash: [0; 32],
            candidate: Block {
                header: commons::Header {
                    version: 0,
                    height: ru.round,
                    timestamp: 0,
                    gas_limit: 0,
                    prev_block_hash: [0; 32],
                    seed: [0; 32],
                    generator_bls_pubkey: [0; 32],
                    state_hash: [0; 32],
                    hash: hash.into(),
                },
                txs: vec![],
            },
            signed_hash: [0; 32],
        };

        Message::from_newblock(
            Header {
                pubkey_bls: ru.pubkey_bls,
                round: ru.round,
                block_hash: hash.into(),
                step,
            },
            a,
        )
    }
}
