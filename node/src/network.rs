// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use std::{any, default};

use crate::{BoxedFilter, Message};
use async_trait::async_trait;
use kadcast::config::Config;
use kadcast::{MessageInfo, Peer};
use node_data::message::Metadata;
use node_data::message::{AsyncQueue, Topics};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::time::{self, Instant};
use tracing::{debug, error, info, trace};

mod frame;

type RoutesList<const N: usize> = [Option<AsyncQueue<Message>>; N];
type FilterList<const N: usize> = [Option<BoxedFilter>; N];

pub struct Listener<const N: usize> {
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
}

impl<const N: usize> Listener<N> {
    fn reroute(&self, topic: u8, msg: Message) -> anyhow::Result<()> {
        let routes = self.routes.clone();

        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic as usize) {
                queue.send(msg.clone()).await;
            };
        });

        Ok(())
    }

    fn call_filters(
        &self,
        topic: impl Into<u8>,
        msg: &Message,
    ) -> anyhow::Result<()> {
        let topic = topic.into() as usize;

        match self.filters.try_write()?.get_mut(topic) {
            Some(Some(f)) => f.filter(msg),
            _ => Ok(()),
        }
    }
}

impl<const N: usize> kadcast::NetworkListen for Listener<N> {
    fn on_message(&self, blob: Vec<u8>, md: MessageInfo) {
        match frame::Pdu::decode(&mut &blob.to_vec()[..]) {
            Ok(d) => {
                let mut msg = d.payload;

                // Update Transport Data
                msg.metadata = Some(Metadata {
                    height: md.height(),
                    src_addr: md.src(),
                });

                // Allow upper layers to fast-discard a message before queueing
                if let Err(e) = self.call_filters(msg.topic(), &msg) {
                    info!("discard message due to {:?}", e);
                    return;
                }

                // Reroute message to the upper layer
                if let Err(e) = self.reroute(msg.topic().into(), msg) {
                    error!("could not reroute due to {:?}", e);
                }
            }
            Err(err) => {
                // Dump message blob and topic number
                error!(
                    "err: {:?}, msg_topic: {:?}",
                    err,
                    blob.get(node_data::message::TOPIC_FIELD_POS),
                );
            }
        };
    }
}

pub struct Kadcast<const N: usize> {
    peer: Peer,
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
    conf: Config,

    counter: AtomicU64,
}

impl<const N: usize> Kadcast<N> {
    pub fn new(conf: Config) -> Self {
        const INIT: Option<AsyncQueue<Message>> = None;
        let routes = Arc::new(RwLock::new([INIT; N]));

        const INIT_FN: Option<BoxedFilter> = None;
        let filters = Arc::new(RwLock::new([INIT_FN; N]));

        Kadcast {
            routes: routes.clone(),
            filters: filters.clone(),
            peer: Peer::new(conf.clone(), Listener { routes, filters })
                .unwrap(),
            conf,
            counter: AtomicU64::new(0),
        }
    }

    pub fn route_internal(&self, msg: Message) {
        let topic = msg.header.topic as usize;
        let routes = self.routes.clone();

        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic) {
                queue.send(msg.clone()).await;
            };
        });
    }

    /// Removes a route, if exists, for a given topic.
    async fn remove_route(&mut self, topic: u8) -> anyhow::Result<()> {
        let mut guard = self.routes.write().await;

        match guard.get_mut(topic as usize) {
            Some(Some(_)) => {
                guard[topic as usize] = None;
                Ok(())
            }
            _ => {
                anyhow::bail!("route not registered for {:?} topic", topic)
            }
        }
    }

    pub async fn alive_nodes(&self, amount: usize) -> Vec<SocketAddr> {
        self.peer.alive_nodes(amount).await
    }

    pub fn conf(&self) -> &Config {
        &self.conf
    }
}

#[async_trait]
impl<const N: usize> crate::Network for Kadcast<N> {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        let height = match msg.metadata {
            Some(Metadata { height: 0, .. }) => return Ok(()),
            Some(Metadata { height, .. }) => Some(height - 1),
            None => None,
        };

        let encoded = frame::Pdu::encode(msg, 0).map_err(|err| {
            error!("could not encode message {:?}: {}", msg, err);
            anyhow::anyhow!("failed to broadcast: {}", err)
        })?;

        trace!("broadcasting msg ({:?})", msg.header.topic);
        self.peer.broadcast(&encoded, height).await;

        Ok(())
    }

    /// Sends an encoded message to a given peer.
    async fn send_to_peer(
        &self,
        msg: &Message,
        recv_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let encoded = frame::Pdu::encode(
            msg,
            self.counter.fetch_add(1, Ordering::SeqCst),
        )
        .map_err(|err| anyhow::anyhow!("failed to send_to_peer: {}", err))?;

        info!(
            "sending msg ({:?}) to peer {:?}",
            msg.header.topic, recv_addr
        );

        self.peer.send(&encoded, recv_addr).await;

        Ok(())
    }

    /// Sends to random set of alive peers.
    async fn send_to_alive_peers(
        &self,
        msg: &Message,
        amount: usize,
    ) -> anyhow::Result<()> {
        let encoded = frame::Pdu::encode(msg, 0)
            .map_err(|err| anyhow::anyhow!("failed to encode: {}", err))?;

        for recv_addr in self.peer.alive_nodes(amount).await {
            trace!(
                "sending msg ({:?}) to peer {:?}",
                msg.header.topic,
                recv_addr
            );

            self.peer.send(&encoded, recv_addr).await;
        }

        Ok(())
    }

    /// Route any message of the specified type to this queue.
    async fn add_route(
        &mut self,
        topic: u8,
        queue: AsyncQueue<Message>,
    ) -> anyhow::Result<()> {
        let mut guard = self.routes.write().await;

        let mut route = guard
            .get_mut(topic as usize)
            .ok_or_else(|| anyhow::anyhow!("topic out of range: {}", topic))?;

        debug_assert!(route.is_none(), "topic already registered");

        *route = Some(queue);

        Ok(())
    }

    async fn send_and_wait(
        &mut self,
        request_msg: &Message,
        response_msg_topic: Topics,
        timeout_millis: u64,
        recv_peers_count: usize,
    ) -> anyhow::Result<Message> {
        self.remove_route(response_msg_topic.into()).await;

        let res = {
            let queue = AsyncQueue::default();
            // register a temporary route that will be unregister on drop
            self.add_route(response_msg_topic.into(), queue.clone())
                .await;

            self.send_to_alive_peers(request_msg, recv_peers_count)
                .await?;

            let deadline =
                Instant::now() + Duration::from_millis(timeout_millis);

            // Wait for a response message or a timeout
            match time::timeout_at(deadline, queue.recv()).await {
                // Got a response message
                Ok(Ok(msg)) => Ok(msg),
                // Failed to receive a response message
                Ok(Err(_)) => anyhow::bail!("failed to receive"),
                // Timeout expired
                Err(_) => anyhow::bail!("timeout err"),
            }
        };

        self.remove_route(response_msg_topic.into()).await;
        res
    }

    async fn add_filter(
        &mut self,
        msg_type: u8,
        filter_fn: BoxedFilter,
    ) -> anyhow::Result<()> {
        let mut guard = self.filters.write().await;

        let mut filter = guard
            .get_mut(msg_type as usize)
            .expect("should be valid type");

        *filter = Some(filter_fn);

        Ok(())
    }

    fn get_info(&self) -> anyhow::Result<String> {
        Ok(self.conf.public_address.to_string())
    }
}
