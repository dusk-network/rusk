// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Database;
use crate::errors::ConsensusError;
use crate::execution_ctx::ExecutionCtx;
use crate::operations::Operations;
use crate::{proposal, ratification, validation};
use node_data::message::Message;
use node_data::StepName;
use tracing::{debug, trace};

macro_rules! await_step {
    ($e:expr, $n:ident ( $($args:expr), *)) => {
        {
           match $e {
                Step::Proposal(p) => p.$n($($args,)*).await,
                Step::Validation(p) => p.$n($($args,)*).await,
                Step::Ratification(p) => p.$n($($args,)*).await,
            }
        }
    };
}

pub enum Step<T: Operations, D: Database> {
    Proposal(proposal::step::ProposalStep<T, D>),
    Validation(validation::step::ValidationStep<T>),
    Ratification(ratification::step::RatificationStep),
}

impl<T: Operations + 'static, D: Database + 'static> Step<T, D> {
    pub fn to_step_name(&self) -> StepName {
        match self {
            Step::Proposal(_) => StepName::Proposal,
            Step::Validation(_) => StepName::Validation,
            Step::Ratification(_) => StepName::Ratification,
        }
    }

    pub async fn reinitialize(
        &mut self,
        msg: Message,
        round: u64,
        iteration: u8,
    ) {
        trace!(event = "init step", msg = format!("{:#?}", msg),);

        await_step!(self, reinitialize(msg, round, iteration))
    }

    pub async fn run(
        &mut self,
        mut ctx: ExecutionCtx<'_, T, D>,
    ) -> Result<Message, ConsensusError> {
        ctx.set_start_time();

        let timeout = ctx.iter_ctx.get_timeout(ctx.step_name());
        debug!(event = "execute_step", ?timeout);

        // Execute step
        await_step!(self, run(ctx))
    }
}
