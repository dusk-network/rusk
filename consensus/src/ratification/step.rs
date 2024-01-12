// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::execution_ctx::ExecutionCtx;
use crate::operations::Operations;
use std::marker::PhantomData;

use crate::msg_handler::{HandleMsgOutput, MsgHandler};
use crate::ratification::handler;
use node_data::ledger::to_str;
use node_data::message;
use node_data::message::payload::{Ratification, ValidationResult};
use node_data::message::{AsyncQueue, Message, Payload, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;

use tracing::{error, info, Instrument};

pub struct RatificationStep<T, DB> {
    handler: Arc<Mutex<handler::RatificationHandler>>,

    _executor: Arc<Mutex<T>>,

    marker: PhantomData<DB>,
}

impl<T: Operations + 'static, DB: Database> RatificationStep<T, DB> {
    pub async fn try_vote(
        ru: &RoundUpdate,
        iteration: u8,
        result: &ValidationResult,
        outbound: AsyncQueue<Message>,
    ) -> Message {
        let hdr = message::Header {
            pubkey_bls: ru.pubkey_bls.clone(),
            round: ru.round,
            iteration,
            block_hash: result.hash,
            topic: Topics::Ratification,
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

        // Publish ratification vote
        info!(event = "send_vote", validation_bitset = result.sv.bitset);

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
            _executor: executor,
            marker: PhantomData,
        }
    }

    pub async fn reinitialize(
        &mut self,
        msg: &Message,
        round: u64,
        iteration: u8,
    ) {
        let mut handler = self.handler.lock().await;
        handler.reset(iteration);

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
            iter = iteration,
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
            .get_current_committee()
            .expect("committee to be created before run");

        if ctx.am_member(committee) {
            let mut handler = self.handler.lock().await;
            let hash = to_str(&handler.validation_result().hash);

            let vote_msg = Self::try_vote(
                &ctx.round_update,
                ctx.iteration,
                handler.validation_result(),
                ctx.outbound.clone(),
            )
            .instrument(tracing::info_span!("ratification", hash,))
            .await;

            // Collect my own vote
            let res = handler
                .collect(vote_msg, &ctx.round_update, committee)
                .await?;
            if let HandleMsgOutput::Ready(m) = res {
                return Ok(m);
            }
        }

        // handle queued messages for current round and step.
        if let Some(m) = ctx.handle_future_msgs(self.handler.clone()).await {
            return Ok(m);
        }

        ctx.event_loop(self.handler.clone()).await
    }

    pub fn name(&self) -> &'static str {
        "ratification"
    }
}
