// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;

use crate::frame::Frame;
use tokio::sync::oneshot;
use tracing::trace;
use crate::selection;
use crate::{firststep, secondstep};

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

pub enum Phase{
    Selection(selection::step::Selection),
    Reduction1(firststep::step::Reduction),
    Reduction2(secondstep::step::Reduction),
}

impl Phase {
    pub fn initialize(&mut self, frame: &Frame, round: u64, step: u8) {
        trace!("init phase:{} with frame {:?} at round:{} step:{}", self.name(),frame, round, step);
        call_phase!(self, initialize(frame))
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        trace!("running phase:{} round:{:?} step:{}", self.name(), ru, step);
        await_phase!(self, run(ctx_recv, ru, step))
    }

    pub fn close(&self)  {
        call_phase!(self, close())
    }

    fn name(&self) -> String {
        call_phase!(self, name())
    }
}
