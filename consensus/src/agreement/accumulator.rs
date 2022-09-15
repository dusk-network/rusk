// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::messages::Message;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

pub(crate) struct Accumulator {
    task: JoinHandle<()>,
}

impl Accumulator {
    pub fn new(_workers_amount: usize, _votes_tx: Sender<Message>) -> Self {
        let handle = tokio::spawn(async move {
            // TODO:
        });

        //TODO: Run workers

        Self { task: handle }
    }
    /* TODO
    pub fn process(&mut self, _msg: Message) {}

    fn verify(_msg: Message) {
        // TODO: generate committee
    }
     */
}

impl Drop for Accumulator {
    fn drop(&mut self) {
        self.task.abort();
    }
}
