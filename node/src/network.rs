// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::net::{AddrParseError, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use crate::{BoxedFilter, Message};
use async_trait::async_trait;
use kadcast::config::Config;
use kadcast::{MessageInfo, Peer};
use node_data::message::Metadata;
use node_data::message::{AsyncQueue, Topics};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::time::{self, Instant};
use tracing::{error, info, trace, warn};

mod frame;

const MAX_PENDING_SENDERS: u64 = 1000;

type RoutesList<const N: usize> = [Option<AsyncQueue<Message>>; N];
type FilterList<const N: usize> = [Option<BoxedFilter>; N];

pub struct Listener<const N: usize> {
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,

    /// Number of awaiting senders.
    pending_senders: Arc<AtomicU64>,
}

impl<const N: usize> Listener<N> {
    fn reroute(&self, topic: u8, msg: Message) -> anyhow::Result<()> {
        if self.pending_senders.fetch_add(1, Ordering::Relaxed)
            >= MAX_PENDING_SENDERS
        {
            // High value of this field means either a message consumer is
            // blocked or it's too slow on processing a wire msg
            self.pending_senders.store(0, Ordering::Relaxed);
            warn!("too many sender jobs: {}", MAX_PENDING_SENDERS);
        }

        let counter = self.pending_senders.clone();
        let routes = self.routes.clone();

        // Sender task
        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic as usize) {
                if let Err(e) = queue.send(msg).await {
                    error!("Unable to reroute message with topic {topic}: {e}");
                };
            };

            counter.fetch_sub(1, Ordering::Relaxed);
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
                    info!("discard message due to {e}");
                    return;
                }

                // Reroute message to the upper layer
                if let Err(e) = self.reroute(msg.topic().into(), msg) {
                    error!("could not reroute due to {e}");
                }
            }
            Err(err) => {
                // Dump message blob and topic number
                let topic = blob.get(node_data::message::TOPIC_FIELD_POS);
                error!("err: {err}, msg_topic: {topic:?}",);
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
    pub fn new(conf: Config) -> Result<Self, AddrParseError> {
        const INIT: Option<AsyncQueue<Message>> = None;
        let routes = Arc::new(RwLock::new([INIT; N]));

        const INIT_FN: Option<BoxedFilter> = None;
        let filters = Arc::new(RwLock::new([INIT_FN; N]));

        info!(
            "Loading network with public_address {} and private_address {:?}",
            &conf.public_address, &conf.listen_address
        );
        let listener = Listener {
            routes: routes.clone(),
            filters: filters.clone(),
            pending_senders: Arc::new(AtomicU64::new(0)),
        };
        let peer = Peer::new(conf.clone(), listener)?;

        Ok(Kadcast {
            routes,
            filters,
            peer,
            conf,
            counter: AtomicU64::new(0),
        })
    }

    pub fn route_internal(&self, msg: Message) {
        let topic = msg.topic() as usize;
        let routes = self.routes.clone();

        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic) {
                if let Err(e) = queue.send(msg.clone()).await {
                    error!("Unable to route_internal message with topic {topic}: {e}");
                };
            };
        });
    }

    /// Removes a route, if exists, for a given topic.
    async fn remove_route(&mut self, topic: u8) {
        let mut guard = self.routes.write().await;

        if let Some(Some(_)) = guard.get_mut(topic as usize) {
            guard[topic as usize] = None;
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
            error!("could not encode message {msg:?}: {err}");
            anyhow::anyhow!("failed to broadcast: {err}")
        })?;

        trace!("broadcasting msg ({:?})", msg.topic());
        self.peer.broadcast(&encoded, height).await;

        Ok(())
    }

    /// Sends an encoded message to a given peer.
    async fn send_to_peer(
        &self,
        msg: &Message,
        recv_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        // rnd_count is added to bypass kadcast dupemap
        let rnd_count = self.counter.fetch_add(1, Ordering::SeqCst);
        let encoded = frame::Pdu::encode(msg, rnd_count)
            .map_err(|err| anyhow::anyhow!("failed to send_to_peer: {err}"))?;
        let topic = msg.topic();

        info!("sending msg ({topic:?}) to peer {recv_addr}");

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
            .map_err(|err| anyhow::anyhow!("failed to encode: {err}"))?;
        let topic = msg.topic();

        for recv_addr in self.peer.alive_nodes(amount).await {
            trace!("sending msg ({topic:?}) to peer {recv_addr}");

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

        let route = guard
            .get_mut(topic as usize)
            .ok_or_else(|| anyhow::anyhow!("topic out of range: {topic}"))?;

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
            let queue = AsyncQueue::unbounded();
            // register a temporary route that will be unregister on drop
            self.add_route(response_msg_topic.into(), queue.clone())
                .await?;

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

        let filter = guard
            .get_mut(msg_type as usize)
            .expect("should be valid type");

        *filter = Some(filter_fn);

        Ok(())
    }

    fn get_info(&self) -> anyhow::Result<String> {
        Ok(self.conf.public_address.to_string())
    }
}
