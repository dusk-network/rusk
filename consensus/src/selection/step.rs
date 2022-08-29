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
        committee: Committee,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        if committee.am_member() {
            // TODO: GenerateBlock
            // TODO: Publish NewBlock message
            // TODO: Pass the NewBlock to this phase event loop
        }

        // TODO: drain queued messages

        // TODO: event_loop to borrow committee
        event_loop(&mut self.handler, &mut self.msg_rx, ctx_recv, ru, step).await
    }

    pub fn name(&self) -> String {
        String::from("selection")
    }

    pub fn get_committee_size(&self) -> usize {
        COMMITTEE_SIZE
    }
}

impl Drop for Selection {
    fn drop(&mut self) {
        trace!("cleanup");
    }
}
