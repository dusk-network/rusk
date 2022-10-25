// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use consensus::util::pending_queue::PendingQueue;
use kadcast::config::Config;
use kadcast::{MessageInfo, NetworkListen, Peer};

use crate::wire;

pub async fn run_main_loop(
    conf: Config,
    inbound: PendingQueue,
    outbound: PendingQueue,
    agr_inbound: PendingQueue,
    agr_outbound: PendingQueue,
) {
    let peer = Peer::new(
        conf,
        Reader {
            inbound,
            agr_inbound,
        },
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

struct Reader {
    inbound: PendingQueue,
    agr_inbound: PendingQueue,
}

impl NetworkListen for Reader {
    fn on_message(&self, message: Vec<u8>, _md: MessageInfo) {
        let decoded = wire::Frame::decode(message.to_vec());
        let msg = decoded.get_msg().clone();

        // Delegate message to the proper queue for further processing.
        if decoded.get_topic() == consensus::commons::Topics::Agreement as u8 {
            if let Err(e) = self.agr_inbound.try_send(msg) {
                tracing::error!("could not delegate msg due to: {:#}", e);
            }
        } else {
            if let Err(e) = self.inbound.try_send(msg) {
                tracing::error!("could not delegate msg due to: {:#}", e);
            }
        }
    }
}
