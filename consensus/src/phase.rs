// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{RoundUpdate, SelectError};
use crate::consensus::Context;

use crate::frame::Frame;
use async_trait::async_trait;
use tokio::sync::oneshot;

#[async_trait]
pub trait Phase {
    // Initialize a new phase execution.
    fn initialize(&mut self, pkg: &Frame);

    // run executes a phase and returns a Frame to be used as input for next phase.
    async fn run(
        &mut self,
        ctx_recv: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Frame, SelectError>;

    fn name(&self) -> String;
}
