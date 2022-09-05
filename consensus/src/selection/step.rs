use hex::ToHex;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::{event_loop, MsgHandler};
use crate::frame::Frame;
use crate::messages::{payload::NewBlock, Header, Message};
use crate::queue::Queue;
use crate::selection::handler;
use crate::user::committee::Committee;
use crate::user::provisioners::PublicKey;
use sha3::{Digest, Sha3_256};
use tokio::sync::{mpsc, oneshot};
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

    pub fn initialize(&mut self, _frame: &Frame) {
        // TODO:
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        inbound_msgs: &mut mpsc::Receiver<Message>,
        outbound_msgs: &mut mpsc::Sender<Message>,
        committee: Committee,
        future_msgs: &mut Queue<Message>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        if committee.am_member() {
            let msg = self.generate_candidate(committee.get_my_pubkey(), ru, step);

            // Broadcast the candidate block for this round/iteration.
            if let Err(e) = outbound_msgs.send(msg.clone()).await {
                error!("could not send newblock msg due to {:?}", e);
            }

            // re-handling my own candidate to ensure both verification and generate-block procedures are compatible
            match self.handler.handle(msg, ru, step, &committee) {
                Ok(f) => return Ok(f),
                Err(e) => error!("invalid candidate generated due to {:?}", e),
            };
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

        let hash = hasher.finalize();

        info!(
            "generate candidate block hash={} round={}, step={}, bls_key={}",
            hash.as_slice().encode_hex::<String>(),
            ru.round,
            step,
            pubkey.encode_short_hex()
        );

        let a = NewBlock {
            prev_hash: [0; 32],
            candidate: Block::default(),
            signed_hash: [0; 32],
        };

        Message::new_newblock(
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
