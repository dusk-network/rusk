// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{net::IpAddr, sync::Arc};

use crate::{utils::PendingQueue, Message};
use async_trait::async_trait;
use kadcast::{config::Config, MessageInfo, Peer};
use tokio::sync::RwLock;

mod frame;

type RoutesList = Vec<Option<PendingQueue<Message>>>;

pub struct Listener {
    routes: Arc<RwLock<RoutesList>>,
}

impl Listener {
    fn reroute(
        &self,
        topic: impl Into<u8>,
        msg: Message,
    ) -> anyhow::Result<()> {
        let topic = topic.into() as usize;

        _ = match self.routes.try_read()?.get(topic) {
            Some(Some(r)) => r.try_send(msg),
            _ => Ok(()),
        };

        anyhow::Ok(())
    }
}

impl kadcast::NetworkListen for Listener {
    fn on_message(&self, message: Vec<u8>, md: MessageInfo) {
        // TODO: Decode message
        if let Err(e) = self.reroute(0, Message::default()) {
            tracing::error!("could not dispatch {:?}", e);
        }
    }
}

pub struct Kadcast {
    peer: Peer,
    routes: Arc<RwLock<RoutesList>>,
}

impl Kadcast {
    pub fn new(conf: Config) -> Self {
        let routes = Arc::new(RwLock::new(vec![None; 255]));
        Kadcast {
            routes: routes.clone(),
            peer: Peer::new(conf, Listener { routes }).unwrap(),
        }
    }
}

#[async_trait]
impl crate::Network for Kadcast {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        // Sample broadcast
        self.peer.broadcast(&[0u8; 8], Some(0)).await;

        anyhow::Ok(())
    }

    async fn repropagate(
        &self,
        msg: &Message,
        from_height: u8,
    ) -> anyhow::Result<()> {
        anyhow::Ok(())
    }

    async fn send(
        &self,
        msg: &Message,
        dst: Vec<String>,
    ) -> anyhow::Result<()> {
        self.peer
            .send(
                &[0u8; 8],
                std::net::SocketAddr::new(
                    IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                    8080,
                ),
            )
            .await;

        anyhow::Ok(())
    }

    /// Route  any message of the specified type to this queue.
    async fn add_route(
        &mut self,
        msg_type: u8,
        queue: PendingQueue<Message>,
    ) -> anyhow::Result<()> {
        if let Some(q) = self.routes.write().await.get_mut(msg_type as usize) {
            *q = Some(queue)
        }
        anyhow::Ok(())
    }
}
