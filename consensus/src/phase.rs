// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::ConsensusError;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::Message;

use crate::user::committee::Committee;

use crate::{firststep, secondstep, selection};

use tracing::trace;

macro_rules! await_phase {
    ($e:expr, $n:ident ( $($args:expr), *)) => {
        {
           match $e {
                Phase::Selection(p) => p.$n($($args,)*).await,
                Phase::Reduction1(p) => p.$n($($args,)*).await,
                Phase::Reduction2(p) => p.$n($($args,)*).await,
            }
        }
    };
}

macro_rules! call_phase {
    ($e:expr, $n:ident ( $($args:expr), *)) => {
        {
           match $e {
                Phase::Selection(p) => p.$n($($args,)*),
                Phase::Reduction1(p) => p.$n($($args,)*),
                Phase::Reduction2(p) => p.$n($($args,)*),
            }
        }
    };
}

pub enum Phase {
    Selection(selection::step::Selection),
    Reduction1(firststep::step::Reduction),
    Reduction2(secondstep::step::Reduction),
}

impl Phase {
    pub fn initialize(&mut self, msg: &Message, round: u64, step: u8) {
        trace!(
            "init phase:{} with msg {:?} at round:{} step:{}",
            self.name(),
            msg,
            round,
            step
        );
        call_phase!(self, initialize(msg))
    }

    pub async fn run(&mut self, ctx: ExecutionCtx<'_>) -> Result<Message, ConsensusError> {
        tracing::info!("event: execute, timeout: {}", self.get_timeout());

        let size = call_phase!(self, get_committee_size());

        // Perform deterministic_sortition to generate committee of size=N.
        // The extracted members are the provisioners eligible to vote on this particular round and step.
        // In the context of Selection phase, the extracted member is the one eligible to generate the candidate block.
        let step_committee = Committee::new(
            ctx.round_update.pubkey_bls.clone(),
            ctx.provisioners,
            ctx.get_sortition_config(size),
        );

        await_phase!(self, run(ctx, step_committee))
    }

    fn name(&self) -> &'static str {
        call_phase!(self, name())
    }

    fn get_timeout(&self) -> u64 {
        call_phase!(self, get_timeout())
    }
}
