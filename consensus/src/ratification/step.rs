// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use node_data::message::payload::{self, QuorumType, ValidationResult};
use node_data::message::{AsyncQueue, Message, Payload, SignedStepMessage};
use node_data::{get_current_timestamp, message};
use tokio::sync::Mutex;
use tracing::{info, warn, Instrument};

use crate::commons::{Database, RoundUpdate};
use crate::config::is_emergency_iter;
use crate::execution_ctx::ExecutionCtx;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::operations::Operations;
use crate::ratification::handler;

pub struct RatificationStep {
    handler: Arc<Mutex<handler::RatificationHandler>>,
}

impl RatificationStep {
    pub async fn try_vote(
        ru: &RoundUpdate,
        iteration: u8,
        result: &ValidationResult,
        outbound: AsyncQueue<Message>,
    ) -> Message {
        // Sign and construct ratification message
        let ratification =
            self::build_ratification_payload(ru, iteration, result);

        let vote = ratification.vote;
        let msg = Message::from(ratification);

        let is_emergency = is_emergency_iter(iteration);

        if result.quorum() == QuorumType::Valid || !is_emergency {
            // Publish ratification vote
            info!(
              event = "Cast vote",
              step = "Ratification",
              info = ?msg.header,
              vote = ?vote,
              validation_bitset = result.sv().bitset
            );

            // Publish
            outbound.try_send(msg.clone());
        }

        msg
    }
}

pub fn build_ratification_payload(
    ru: &RoundUpdate,
    iteration: u8,
    result: &ValidationResult,
) -> payload::Ratification {
    let header = message::ConsensusHeader {
        prev_block_hash: ru.hash(),
        round: ru.round,
        iteration,
    };

    let sign_info = message::SignInfo::default();
    let mut ratification = message::payload::Ratification {
        header,
        vote: *result.vote(),
        sign_info,
        validation_result: result.clone(),
        timestamp: get_current_timestamp(),
    };
    ratification.sign(&ru.secret_key, ru.pubkey_bls.inner());
    ratification
}

impl RatificationStep {
    pub(crate) fn new(
        handler: Arc<Mutex<handler::RatificationHandler>>,
    ) -> Self {
        Self { handler }
    }

    pub async fn reinitialize(
        &mut self,
        msg: Message,
        round: u64,
        iteration: u8,
    ) {
        let mut handler = self.handler.lock().await;

        // The Validation output must be the vote to cast on the Ratification.
        // There are these possible outputs:
        //  - Quorum on Valid Candidate
        //  - (unsupported) Quorum on Invalid Candidate
        //  - Quorum on Timeout
        //  - No Quorum (Validation step time-ed out)
        match msg.payload {
            Payload::ValidationResult(p) => handler.reset(iteration, *p),
            _ => handler.reset(iteration, Default::default()),
        }

        tracing::debug!(
            event = "init",
            name = self.name(),
            round = round,
            iter = iteration,
            vote = ?handler.validation_result().vote(),
            fsv_bitset = handler.validation_result().sv().bitset,
            quorum_type = ?handler.validation_result().quorum()
        )
    }

    pub async fn run<T: Operations + 'static, DB: Database>(
        &mut self,
        mut ctx: ExecutionCtx<'_, T, DB>,
    ) -> Message {
        let committee = ctx
            .get_current_committee()
            .expect("committee to be created before run");

        let generator = ctx.get_curr_generator();

        if ctx.am_member(committee) {
            let mut handler = self.handler.lock().await;
            let vote = handler.validation_result().vote();

            let vote_msg = Self::try_vote(
                &ctx.round_update,
                ctx.iteration,
                handler.validation_result(),
                ctx.outbound.clone(),
            )
            .instrument(tracing::info_span!("ratification", ?vote))
            .await;

            // Collect my own vote
            match handler
                .collect(
                    vote_msg,
                    &ctx.round_update,
                    committee,
                    generator,
                    &ctx.iter_ctx.committees,
                )
                .await
            {
                Ok(StepOutcome::Ready(m)) => return m,
                Ok(_) => {}
                Err(e) => warn!("Error collecting own vote: {e:?}"),
            }
        }

        // handle queued messages for current round and step.
        match ctx.handle_future_msgs(self.handler.clone()).await {
            StepOutcome::Ready(m) => m,
            StepOutcome::Pending => {
                ctx.event_loop(self.handler.clone(), None).await
            }
        }
    }

    pub fn name(&self) -> &'static str {
        "ratification"
    }
}
