// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;
use crate::frame::Frame;
use crate::messages::MsgNewBlock;
use crate::selection::handler;

use crate::event_loop::event_loop;

use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tracing::trace;

pub const COMMITTEE_SIZE: usize = 1;

pub struct Selection {
    msg_rx: Receiver<MsgNewBlock>,
    handler: handler::Selection,
}

impl Selection {
    pub fn new(msg_rx: Receiver<MsgNewBlock>) -> Self {
        Self {
            msg_rx,
            handler: handler::Selection {},
        }
    }

    pub fn initialize(&mut self, _frame: &Frame) {
        // TODO:
    }

    pub async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        provionsers: &mut Provisioners,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        //TODO: Perform sortition

        // Perform sortition to generate committee of size=1 for Selection step.
        // The extracted member is the Block Generator of current consensus iteration.
        let step_committee = Committee::new(
            ru.pubkey_bls.clone(),
            provionsers,
            ru.seed,
            ru.round,
            step,
            COMMITTEE_SIZE,
        );

        if step_committee.am_member() {
            // TODO: GenerateBlock
            // TODO: Publish Candidate Block
        }

        // TODO: Move step_committee to event_loop

        event_loop(&mut self.handler, &mut self.msg_rx, ctx_recv, ru, step).await
    }

    pub fn name(&self) -> String {
        String::from("selection")
    }

    pub fn close(&self) {
        // TODO:
    }
}

impl Drop for Selection {
    fn drop(&mut self) {
        trace!("cleanup");
    }
}
