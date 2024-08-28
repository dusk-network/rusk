// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database};
use crate::config::CONSENSUS_MAX_ITER;
use crate::execution_ctx::ExecutionCtx;
use crate::operations::Operations;
use crate::user::committee::Committee;
use crate::{proposal, ratification, validation};
use node_data::message::Message;
use node_data::StepName;
use tracing::{debug, trace};

macro_rules! await_phase {
    ($e:expr, $n:ident ( $($args:expr), *)) => {
        {
           match $e {
                Phase::Proposal(p) => p.$n($($args,)*).await,
                Phase::Validation(p) => p.$n($($args,)*).await,
                Phase::Ratification(p) => p.$n($($args,)*).await,
            }
        }
    };
}

pub enum Phase<T: Operations, D: Database> {
    Proposal(proposal::step::ProposalStep<T, D>),
    Validation(validation::step::ValidationStep<T>),
    Ratification(ratification::step::RatificationStep),
}

impl<T: Operations + 'static, D: Database + 'static> Phase<T, D> {
    pub fn to_step_name(&self) -> StepName {
        match self {
            Phase::Proposal(_) => StepName::Proposal,
            Phase::Validation(_) => StepName::Validation,
            Phase::Ratification(_) => StepName::Ratification,
        }
    }

    pub async fn reinitialize(
        &mut self,
        msg: Message,
        round: u64,
        iteration: u8,
    ) {
        trace!(event = "init step", msg = format!("{:#?}", msg),);

        await_phase!(self, reinitialize(msg, round, iteration))
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, T>,
    ) -> Result<Message, ConsensusError> {
        ctx.set_start_time();

        let step_name = ctx.step_name();
        let timeout = ctx.iter_ctx.get_timeout(ctx.step_name());
        debug!(event = "execute_step", ?timeout);

        let exclusion = match step_name {
            StepName::Proposal => vec![],
            _ => {
                let mut exclusion_list = vec![];
                let generator = ctx
                    .iter_ctx
                    .get_generator(ctx.iteration)
                    .expect("Proposal committee to be already generated");

                exclusion_list.push(generator);

                if ctx.iteration < CONSENSUS_MAX_ITER {
                    let next_generator =
                        ctx.iter_ctx.get_generator(ctx.iteration + 1).expect(
                            "Next Proposal committee to be already generated",
                        );

                    exclusion_list.push(next_generator);
                }

                exclusion_list
            }
        };

        // Perform deterministic_sortition to generate committee of size=N.
        // The extracted members are the provisioners eligible to vote on this
        // particular round and step. In the context of Proposal phase,
        // the extracted member is the one eligible to generate the candidate
        // block.
        let step_committee = Committee::new(
            ctx.provisioners,
            &ctx.get_sortition_config(exclusion),
        );

        if let StepName::Proposal = step_name {
            if ctx.iteration < CONSENSUS_MAX_ITER {
                let mut cfg_next_iteration = ctx.get_sortition_config(vec![]);
                cfg_next_iteration.step =
                    StepName::Proposal.to_step(ctx.iteration + 1);

                ctx.save_committee(
                    cfg_next_iteration.step,
                    Committee::new(ctx.provisioners, &cfg_next_iteration),
                );
            }
        }

        debug!(
            event = "committee_generated",
            members = format!("{}", &step_committee)
        );

        ctx.save_committee(ctx.step(), step_committee);

        // Execute step
        await_phase!(self, run(ctx))
    }
}
