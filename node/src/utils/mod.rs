// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use async_channel::TrySendError;

/// PendingQueue is a thin wrapper around async_channel of M.
#[derive(Clone)]
pub struct PendingQueue<M: Clone> {
    receiver: async_channel::Receiver<M>,
    sender: async_channel::Sender<M>,
}

impl<M: Clone> Default for PendingQueue<M> {
    fn default() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self { receiver, sender }
    }
}

impl<M: Clone> PendingQueue<M> {
    pub fn send(&mut self, msg: M) -> async_channel::Send<'_, M> {
        self.sender.send(msg)
    }

    pub fn try_send(&self, msg: M) -> Result<(), TrySendError<M>> {
        self.sender.try_send(msg)
    }

    pub fn recv(&self) -> async_channel::Recv<'_, M> {
        self.receiver.recv()
    }
}
