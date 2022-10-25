// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use consensus::commons::Topics;
use consensus::util::pending_queue::PendingQueue;
use kadcast::config::Config;
use kadcast::{MessageInfo, NetworkListen, Peer};
use consensus::messages::Message;

use crate::wire;

pub async fn run_main_loop(
    conf: Config,
    inbound: PendingQueue,
    outbound: PendingQueue,
    agr_inbound: PendingQueue,
    agr_outbound: PendingQueue,
) {
    // Initialize reader and its dispatcher
    let mut r = Reader::default();
    r.msg_dispatcher.add(Topics::Agreement as usize, agr_inbound);
    r.msg_dispatcher.add_default(inbound);

    let peer = Peer::new(
        conf,
        r,
    );

    // Broadcast outbound messages with a priority to the messages from agreement loop
    loop {
        tokio::select! {
            biased;
            recv = agr_outbound.recv() => {
                if let Ok(msg) = recv {
                    peer.broadcast(&wire::Frame::encode(msg), None).await;
                }
            }
            recv = outbound.recv() => {
                if let Ok(msg) = recv {
                    peer.broadcast(&wire::Frame::encode(msg), None).await;
                }
            }
           
        }
    }
}

#[derive(Default)]
struct Reader {
    pub msg_dispatcher: Dispatcher,
}

impl NetworkListen for Reader {
    fn on_message(&self, message: Vec<u8>, _md: MessageInfo) {
        let decoded = wire::Frame::decode(message.to_vec());
        let msg = decoded.get_msg().clone();

        // Dispatch message to the proper queue for further processing    
        if let Err(e) = self.msg_dispatcher.dispatch(decoded.get_topic() as usize, msg) {
            tracing::error!("could not dispatch {:?}",e);
        }
    }
}
 
/// Implements a simple message dispatcher that delegates a message to the
/// associated queue depending on the topic value read from wire message.
struct Dispatcher {
    queues: Vec<Option<PendingQueue>>,
    default_queue: Option<PendingQueue>,
}

impl Dispatcher {
    fn add(&mut self, topic: usize,queue: PendingQueue) {
        self.queues[topic] = Some(queue);
    }

    fn add_default(&mut self, queue: PendingQueue) {
        self.default_queue = Some(queue);
    }

    fn dispatch(&self, topic: usize, msg: Message) -> Result<(), async_channel::TrySendError<Message>> {
        if topic < self.queues.len() {
            if let Some(q) = &self.queues[topic] {
                return q.try_send(msg);
            }
        }
       
        if let Some(q) = &self.default_queue {
            return q.try_send(msg);
        }

        Err(async_channel::TrySendError::Closed(msg))
    }
}

impl Default for Dispatcher{
    fn default() -> Self {
        Self { queues: vec![None; 255], default_queue: None }
    }
}