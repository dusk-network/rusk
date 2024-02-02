// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Database;
use crate::commons::{RoundUpdate, TimeoutSet};
use std::cmp;

use crate::config::{MAX_STEP_TIMEOUT, TIMEOUT_INCREASE};
use crate::msg_handler::HandleMsgOutput;
use crate::msg_handler::MsgHandler;

use crate::user::committee::Committee;

use crate::{proposal, ratification, validation};
use node_data::bls::PublicKeyBytes;

use node_data::message::Message;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use node_data::StepName;
use tracing::debug;

/// A pool of all generated committees
#[derive(Default)]
pub struct RoundCommittees {
    committees: HashMap<u16, Committee>,
}

impl RoundCommittees {
    pub(crate) fn get_committee(&self, step: u16) -> Option<&Committee> {
        self.committees.get(&step)
    }

    pub(crate) fn get_generator(&self, iter: u8) -> Option<PublicKeyBytes> {
        let step = StepName::Proposal.to_step(iter);
        self.get_committee(step)
            .and_then(|c| c.iter().next().map(|p| *p.bytes()))
    }

    pub(crate) fn get_validation_committee(
        &self,
        iter: u8,
    ) -> Option<&Committee> {
        let step = StepName::Validation.to_step(iter);
        self.get_committee(step)
    }

    pub(crate) fn insert(&mut self, step: u16, committee: Committee) {
        self.committees.insert(step, committee);
    }
}

/// Represents a shared state within a context of the execution of a single
/// iteration.
pub struct IterationCtx<DB: Database> {
    validation_handler: Arc<Mutex<validation::handler::ValidationHandler>>,
    ratification_handler:
        Arc<Mutex<ratification::handler::RatificationHandler>>,
    proposal_handler: Arc<Mutex<proposal::handler::ProposalHandler<DB>>>,

    pub join_set: JoinSet<()>,

    round: u64,
    iter: u8,

    /// Stores any committee already generated in the execution of any
    /// iteration of current round
    pub(crate) committees: RoundCommittees,

    /// Implements the adaptive timeout algorithm
    timeouts: TimeoutSet,
}

impl<D: Database> IterationCtx<D> {
    pub fn new(
        round: u64,
        iter: u8,
        proposal_handler: Arc<Mutex<proposal::handler::ProposalHandler<D>>>,
        validation_handler: Arc<Mutex<validation::handler::ValidationHandler>>,
        ratification_handler: Arc<
            Mutex<ratification::handler::RatificationHandler>,
        >,
        timeouts: TimeoutSet,
    ) -> Self {
        Self {
            round,
            join_set: JoinSet::new(),
            iter,
            proposal_handler,
            validation_handler,
            ratification_handler,
            committees: Default::default(),
            timeouts,
        }
    }

    /// Executed on starting a new iteration, before Proposal step execution
    pub(crate) fn on_begin(&mut self, iter: u8) {
        self.iter = iter;
    }

    /// Executed on closing an iteration, after Ratification step execution
    pub(crate) fn on_close(&mut self) {
        debug!(
            event = "iter completed",
            len = self.join_set.len(),
            round = self.round,
            iter = self.iter,
        );
        self.join_set.abort_all();
    }

    /// Handles an event of a Phase timeout
    pub(crate) fn on_timeout_event(&mut self, step_name: StepName) {
        let curr_step_timeout =
            self.timeouts.get_mut(&step_name).expect("valid timeout");

        *curr_step_timeout =
            cmp::min(MAX_STEP_TIMEOUT, curr_step_timeout.add(TIMEOUT_INCREASE));
    }

    /// Calculates and returns the adjusted timeout for the specified step
    pub(crate) fn get_timeout(&self, step_name: StepName) -> Duration {
        *self
            .timeouts
            .get(&step_name)
            .expect("valid timeout per step")
    }

    pub(crate) fn get_generator(&self, iter: u8) -> Option<PublicKeyBytes> {
        let step = StepName::Proposal.to_step(iter);
        self.committees
            .get_committee(step)
            .and_then(|c| c.iter().next().map(|p| *p.bytes()))
    }

    /// Collects a message from a past iteration
    pub(crate) async fn collect_past_event(
        &self,
        ru: &RoundUpdate,
        msg: Message,
    ) -> Option<Message> {
        let committee = self.committees.get_committee(msg.get_step())?;
        match msg.topic() {
            node_data::message::Topics::Candidate => {
                let mut handler = self.proposal_handler.lock().await;
                _ = handler.collect_from_past(msg, ru, committee).await;
            }
            node_data::message::Topics::Validation => {
                let mut handler = self.validation_handler.lock().await;
                if let Ok(HandleMsgOutput::Ready(m)) =
                    handler.collect_from_past(msg, ru, committee).await
                {
                    return Some(m);
                }
            }
            node_data::message::Topics::Ratification => {
                let mut handler = self.ratification_handler.lock().await;
                if let Ok(HandleMsgOutput::Ready(m)) =
                    handler.collect_from_past(msg, ru, committee).await
                {
                    return Some(m);
                }
            }
            _ => {}
        };

        None
    }
}

impl<DB: Database> Drop for IterationCtx<DB> {
    fn drop(&mut self) {
        self.on_close();
    }
}
