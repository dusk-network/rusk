// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use crate::commons::{ConsensusError, Database};
use crate::contract_state::Operations;
use crate::execution_ctx::ExecutionCtx;

use node_data::message::Message;
use node_data::StepName;

use crate::user::committee::Committee;

use crate::{proposal, ratification, validation};

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

macro_rules! call_phase {
    ($e:expr, $n:ident ( $($args:expr), *)) => {
        {
           match $e {
                Phase::Proposal(p) => p.$n($($args,)*),
                Phase::Validation(p) => p.$n($($args,)*),
                Phase::Ratification(p) => p.$n($($args,)*),
            }
        }
    };
}

pub enum Phase<T: Operations, D: Database> {
    Proposal(proposal::step::ProposalStep<T, D>),
    Validation(validation::step::ValidationStep<T>),
    Ratification(ratification::step::RatificationStep<T, D>),
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
        msg: &Message,
        round: u64,
        iteration: u8,
    ) {
        trace!(event = "init step", msg = format!("{:#?}", msg),);

        await_phase!(self, reinitialize(msg, round, iteration))
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, D, T>,
    ) -> Result<Message, ConsensusError> {
        debug!(event = "execute_step", timeout = self.get_timeout().as_millis());

        let size = call_phase!(self, get_committee_size());

        let exclusion = match ctx.step_name() {
            StepName::Proposal => None,
            _ => {
                let generator = ctx
                    .iter_ctx
                    .get_generator(ctx.iteration)
                    .expect("Proposal committee to be already generated");
                Some(generator)
            }
        };

        // Perform deterministic_sortition to generate committee of size=N.
        // The extracted members are the provisioners eligible to vote on this
        // particular round and step. In the context of Proposal phase,
        // the extracted member is the one eligible to generate the candidate
        // block.
        let step_committee = Committee::new(
            ctx.provisioners,
            &ctx.get_sortition_config(size, exclusion),
        );

        debug!(
            event = "committee_generated",
            members = format!("{}", &step_committee)
        );

        ctx.save_committee(step_committee);

        await_phase!(self, run(ctx))
    }

    fn get_timeout(&self) -> Duration {
        call_phase!(self, get_timeout())
    }
}
