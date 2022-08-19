// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::phase::*;

use crate::consensus::Context;
use crate::frame::Frame;
use crate::messages::MsgNewBlock;
use crate::selection::handler;

use crate::event_loop::event_loop;
use async_trait::async_trait;

use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tracing::trace;

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
}

impl Drop for Selection {
    fn drop(&mut self) {
        trace!("cleanup");
    }
}

#[async_trait]
impl Phase for Selection {
    fn initialize(&mut self, frame: &Frame) {
        trace!("initializing with frame: {:?}  ", frame);
    }

    async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError> {
        trace!("running {:?} round:{} step:{}", self.name(), ru.round, step);
        event_loop(&mut self.handler, &mut self.msg_rx, ctx_recv, ru, step).await
    }

    fn name(&self) -> String {
        String::from("selection")
    }
}
