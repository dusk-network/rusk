// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::messages::Message;

/// PendingQueue is a thin wrapper around async_channel of Message.
///  It is used for supporting inbound and outbound message queues.
#[derive(Clone)]
pub struct PendingQueue {
    /// Queue name
    id: String,

    /// Receiver/Sender
    receiver: async_channel::Receiver<Message>,
    sender: async_channel::Sender<Message>,

    /// A patch for discarding duplicated messages in mocked test-harness.
    /// This would not be needed once Kadcast integrated.
    duplicates: Vec<Message>,
}

impl PendingQueue {
    pub fn new_with_chan(
        id: &str,
        sender: async_channel::Sender<Message>,
        receiver: async_channel::Receiver<Message>,
    ) -> Self {
        Self {
            id: id.to_string(),
            receiver,
            sender,
            duplicates: vec![],
        }
    }

    pub fn new(id: &str) -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            id: id.to_string(),
            receiver,
            sender,
            duplicates: vec![],
        }
    }

    pub fn is_duplicate(&self, msg: &Message) -> bool {
        for i in self.duplicates.iter() {
            if i.header == msg.header {
                return true;
            }
        }

        return false;
    }

    pub fn send(&mut self, msg: Message) -> async_channel::Send<'_, Message> {
        self.duplicates.push(msg.clone());

        tracing::trace!("sending {:?} by {}", msg, self.id);
        self.sender.send(msg)
    }

    pub fn recv(&self) -> async_channel::Recv<'_, Message> {
        self.receiver.recv()
    }
}
