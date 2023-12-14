// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::config;
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;
use std::marker::PhantomData;

use crate::msg_handler::MsgHandler;
use crate::ratification::handler;
use node_data::ledger::to_str;
use node_data::message;
use node_data::message::payload::{Ratification, ValidationResult};
use node_data::message::{AsyncQueue, Message, Payload, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;

use tracing::{debug, error};

pub struct RatificationStep<T, DB> {
    handler: Arc<Mutex<handler::RatificationHandler>>,

    timeout_millis: u64,
    _executor: Arc<Mutex<T>>,

    marker: PhantomData<DB>,
}

impl<T: Operations + 'static, DB: Database> RatificationStep<T, DB> {
    pub async fn cast_vote(
        &self,
        ru: &RoundUpdate,
        step: u8,
        result: &ValidationResult,
        outbound: AsyncQueue<Message>,
    ) -> Message {
        let hdr = message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            step,
            block_hash: result.hash,
            topic: Topics::Ratification.into(),
        };

        let signature = hdr.sign(&ru.secret_key, ru.pubkey_bls.inner());

        // Sign and construct ratification message
        let msg = Message::new_ratification(
            hdr,
            Ratification {
                signature,
                validation_result: result.clone(),
            },
        );

        debug!(
            event = "voting",
            vtype = "ratification",
            hash = to_str(&result.hash),
            validation_bitset = result.sv.bitset
        );

        // Publish
        outbound.send(msg.clone()).await.unwrap_or_else(|err| {
            error!("could not publish ratification msg {:?}", err)
        });

        msg
    }
}

impl<T: Operations + 'static, DB: Database> RatificationStep<T, DB> {
    pub(crate) fn new(
        executor: Arc<Mutex<T>>,
        handler: Arc<Mutex<handler::RatificationHandler>>,
    ) -> Self {
        Self {
            handler,
            timeout_millis: config::CONSENSUS_TIMEOUT_MS,
            _executor: executor,
            marker: PhantomData,
        }
    }

    pub async fn reinitialize(&mut self, msg: &Message, round: u64, step: u8) {
        let mut handler = self.handler.lock().await;
        handler.reset(step);

        // The Validation output must be the vote to cast on the Ratification.
        // There are these possible outputs:
        //  - Quorum on Valid Candidate
        //  - (unsupported) Quorum on Invalid Candidate
        //  - Quorum on Timeout (NilQuorum)
        //  - No Quorum (Validation step time-ed out)

        if let Payload::ValidationResult(p) = &msg.payload {
            handler.validation_result = p.as_ref().clone();
        }

        tracing::debug!(
            event = "init",
            name = self.name(),
            round = round,
            step = step,
            timeout = self.timeout_millis,
            hash = to_str(&handler.validation_result().hash),
            fsv_bitset = handler.validation_result().sv.bitset,
            quorum_type = format!("{:?}", handler.validation_result().quorum)
        )
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, DB, T>,
    ) -> Result<Message, ConsensusError> {
        let committee = ctx
            .get_committee()
            .expect("committee to be created before run");

        if committee.am_member() {
            let mut handler = self.handler.lock().await;

            let vote_msg = self
                .cast_vote(
                    &ctx.round_update,
                    ctx.step,
                    handler.validation_result(),
                    ctx.outbound.clone(),
                )
                .await;

            // Collect my own vote
            handler
                .collect(vote_msg, &ctx.round_update, ctx.step, committee)
                .await?;
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(self.handler.clone()).await {
            return Ok(m);
        }

        ctx.event_loop(self.handler.clone(), &mut self.timeout_millis)
            .await
    }

    pub fn name(&self) -> &'static str {
        "ratification"
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout_millis
    }

    pub fn get_committee_size(&self) -> usize {
        config::RATIFICATION_COMMITTEE_SIZE
    }
}
