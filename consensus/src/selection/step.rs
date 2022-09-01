use hex::ToHex;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::event_loop::{event_loop, MsgHandler};
use crate::frame::Frame;
use crate::messages::Message;
use crate::queue::Queue;
use crate::selection::handler;
use crate::user::committee::Committee;
use crate::user::provisioners::PublicKey;
use sha3::{Digest, Sha3_256};
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tracing::{info};

pub const COMMITTEE_SIZE: usize = 1;

pub struct Selection {
    msg_rx: Receiver<Message>,
    handler: handler::Selection,
}

impl Selection {
    pub fn new(msg_rx: Receiver<Message>) -> Self {
        Self {
            msg_rx,
            handler: handler::Selection {},
        }
    }

    pub fn initialize(&mut self, _frame: &Frame) {
        // TODO:
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        committee: Committee,
        future_msgs: &mut Queue<Message>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        if committee.am_member() {
            self.generate_candidate(committee.get_my_pubkey(), ru, step);
            // TODO: Publish NewBlock message
            // TODO: Pass the NewBlock to this phase event loop
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
            &mut self.msg_rx,
            ctx_recv,
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
    fn generate_candidate(&self, pubkey: PublicKey, ru: RoundUpdate, step: u8) {
        let mut hasher = Sha3_256::new();
        hasher.update(ru.round.to_le_bytes());
        hasher.update(step.to_le_bytes());

        info!(
            "generate candidate block hash={} round={}, step={}, bls_key={}",
            hasher.finalize().as_slice().encode_hex::<String>(),
            ru.round,
            step,
            pubkey.encode_short_hex()
        );
    }
}
