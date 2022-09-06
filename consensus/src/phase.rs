// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::messages::Message;
use crate::queue::Queue;
use crate::selection;
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::{firststep, secondstep};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tracing::info;

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
        info!(
            "init phase:{} with msg {:?} at round:{} step:{}",
            self.name(),
            msg,
            round,
            step
        );
        call_phase!(self, initialize(msg))
    }

    pub async fn run(
        &mut self,
        provisioners: &mut Provisioners,
        future_msgs: &mut Queue<Message>,
        ctx_recv: &mut oneshot::Receiver<Context>,
        inbound_msgs: &mut Receiver<Message>,
        outbound_msgs: &mut Sender<Message>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Message, SelectError> {
        info!(
            "execute {} round={}, step={}, bls_key={}",
            self.name(),
            ru.round,
            step,
            ru.pubkey_bls.encode_short_hex()
        );

        let size = call_phase!(self, get_committee_size());

        // Perform deterministic_sortition to generate committee of size=N.
        // The extracted members are the provisioners eligible to vote on this particular round and step.
        // In the context of Selection phase, the extracted member is the one eligible to generate the candidate block.
        let step_committee = Committee::new(
            ru.pubkey_bls,
            provisioners,
            sortition::Config(ru.seed, ru.round, step, size),
        );

        await_phase!(
            self,
            run(
                ctx_recv,
                inbound_msgs,
                outbound_msgs,
                step_committee,
                future_msgs,
                ru,
                step
            )
        )
    }

    fn name(&self) -> &'static str {
        call_phase!(self, name())
    }
}
