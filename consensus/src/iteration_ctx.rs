// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

use node_data::bls::PublicKeyBytes;
use node_data::ledger::Seed;
use node_data::message::{Message, Topics};
use node_data::StepName;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::debug;

use crate::commons::{Database, TimeoutSet};
use crate::config::{
    exclude_next_generator, MAX_STEP_TIMEOUT, TIMEOUT_INCREASE,
};
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::{proposal, ratification, validation};

/// A pool of all generated committees
#[derive(Default)]
pub struct RoundCommittees {
    committees: HashMap<u8, Committee>,
}

impl RoundCommittees {
    pub(crate) fn get_committee(&self, step: u8) -> Option<&Committee> {
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

    pub(crate) fn insert(&mut self, step: u8, committee: Committee) {
        self.committees.insert(step, committee);
    }
}

/// Represents a shared state within a context of the execution of a single
/// iteration.
pub struct IterationCtx<DB: Database> {
    validation_handler: Arc<Mutex<validation::handler::ValidationHandler<DB>>>,
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

impl<DB: Database> IterationCtx<DB> {
    pub fn new(
        round: u64,
        iter: u8,
        validation_handler: Arc<
            Mutex<validation::handler::ValidationHandler<DB>>,
        >,
        ratification_handler: Arc<
            Mutex<ratification::handler::RatificationHandler>,
        >,
        proposal_handler: Arc<Mutex<proposal::handler::ProposalHandler<DB>>>,
        timeouts: TimeoutSet,
    ) -> Self {
        Self {
            round,
            join_set: JoinSet::new(),
            iter,
            validation_handler,
            ratification_handler,
            committees: Default::default(),
            timeouts,
            proposal_handler,
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

    fn get_sortition_config(
        &self,
        seed: Seed,
        step_name: StepName,
        exclusion: Vec<PublicKeyBytes>,
    ) -> sortition::Config {
        sortition::Config::new(
            seed, self.round, self.iter, step_name, exclusion,
        )
    }

    pub(crate) fn generate_committee(
        &mut self,
        iteration: u8,
        step_name: StepName,
        provisioners: &Provisioners,
        seed: Seed,
    ) {
        let step = step_name.to_step(iteration);

        // Check if we already generated the committee.
        // This will be usually the case for all Proposal steps after
        // iteration 0
        if self.committees.get_committee(step).is_some() {
            return;
        }

        // For Validation and Ratification steps we need the next-iteration
        // generator for the exclusion list. So we extract, it if necessary.
        //
        // This is not necessary in the last iteration, so we skip it
        if step_name != StepName::Proposal && exclude_next_generator(iteration)
        {
            let prop = StepName::Proposal;
            let next_prop_step = prop.to_step(iteration + 1);

            // Check if this committee has been already generated.
            // This will be typically the case when executing the Ratification
            // step after the Validation one
            if self.committees.get_committee(next_prop_step).is_none() {
                let mut next_cfg =
                    self.get_sortition_config(seed, prop, vec![]);
                next_cfg.step = next_prop_step;

                let next_generator = Committee::new(provisioners, &next_cfg);

                debug!(
                  event = "committee_generated",
                  step = next_cfg.step,
                  config = ?next_cfg,
                  members = format!("{}", &next_generator)
                );

                self.committees.insert(next_prop_step, next_generator);
            }
        }

        // Fill up exclusion list
        //
        // We exclude the generators for the current iteration and the next one
        // to avoid conflict of interests
        let exclusion = match step_name {
            StepName::Proposal => vec![],
            _ => {
                let mut exclusion_list = vec![];
                // Exclude generator for current iteration
                let cur_generator = self
                    .get_generator(iteration)
                    .expect("Proposal committee to be already generated");

                exclusion_list.push(cur_generator);

                // Exclude generator for next iteration
                if exclude_next_generator(iteration) {
                    let next_generator =
                        self.get_generator(iteration + 1).expect(
                            "Next Proposal committee to be already generated",
                        );

                    exclusion_list.push(next_generator);
                }

                exclusion_list
            }
        };

        // Generate the committee for the current step
        // If the step is Proposal, the only extracted member is the generator
        // For Validation and Ratification steps, extracted members are
        // delegated to vote on the candidate block

        let sortition_step = step_name.to_step(iteration);
        let mut config_step =
            self.get_sortition_config(seed, step_name, exclusion);
        config_step.step = sortition_step;
        let step_committee = Committee::new(provisioners, &config_step);

        debug!(
            event = "committee_generated",
            step = config_step.step,
            config = ?config_step,
            members = format!("{}", &step_committee)
        );

        self.committees.insert(step, step_committee);
    }

    pub(crate) fn generate_iteration_committees(
        &mut self,
        iteration: u8,
        provisioners: &Provisioners,
        seed: Seed,
    ) {
        let stepnames = [
            StepName::Proposal,
            StepName::Validation,
            StepName::Ratification,
        ];

        for stepname in &stepnames {
            self.generate_committee(iteration, *stepname, provisioners, seed);
        }
    }

    pub(crate) fn get_generator(&self, iter: u8) -> Option<PublicKeyBytes> {
        let step = StepName::Proposal.to_step(iter);
        self.committees
            .get_committee(step)
            .and_then(|c| c.iter().next())
            .map(|p| *p.bytes())
    }

    /// Collects a message from a past iteration
    pub(crate) async fn process_past_msg(
        &self,
        msg: Message,
    ) -> Option<Message> {
        let committee = self.committees.get_committee(msg.get_step())?;
        let generator = self.get_generator(msg.header.iteration);

        match msg.topic() {
            Topics::Candidate => {
                let mut proposal = self.proposal_handler.lock().await;
                if let Ok(StepOutcome::Ready(m)) =
                    proposal.collect_from_past(msg, committee, generator).await
                {
                    return Some(m);
                }
            }

            Topics::Validation | Topics::ValidationQuorum => {
                let mut validation = self.validation_handler.lock().await;
                if let Ok(StepOutcome::Ready(m)) = validation
                    .collect_from_past(msg, committee, generator)
                    .await
                {
                    return Some(m);
                }
            }

            Topics::Ratification => {
                let mut ratification = self.ratification_handler.lock().await;
                if let Ok(StepOutcome::Ready(m)) = ratification
                    .collect_from_past(msg, committee, generator)
                    .await
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
