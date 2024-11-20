// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::net::{AddrParseError, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{BoxedFilter, Message};
use async_trait::async_trait;
use kadcast::config::Config;
use kadcast::{MessageInfo, Peer};
use metrics::counter;
use node_data::ledger::to_str;
use node_data::message::payload::{GetResource, Inv, Nonce};
use node_data::message::{AsyncQueue, Metadata, PROTOCOL_VERSION};
use node_data::{get_current_timestamp, Serializable};
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};

/// Number of alive peers randomly selected which a `flood_request` is sent to
const REDUNDANCY_PEER_COUNT: usize = 8;

type RoutesList<const N: usize> = [Option<AsyncQueue<Message>>; N];
type FilterList<const N: usize> = [Option<BoxedFilter>; N];

pub struct Listener<const N: usize> {
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
}

impl<const N: usize> Listener<N> {
    fn reroute(&self, topic: u8, msg: Message) {
        let routes = self.routes.clone();
        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic as usize) {
                queue.try_send(msg);
            };
        });
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
        let msg_size = blob.len();
        match Message::read(&mut &blob.to_vec()[..]) {
            Ok(mut msg) => {
                counter!("dusk_bytes_recv").increment(msg_size as u64);
                counter!(format!("dusk_inbound_{:?}_size", msg.topic()))
                    .increment(msg_size as u64);
                counter!(format!("dusk_inbound_{:?}_count", msg.topic()))
                    .increment(1);

                let ray_id = to_str(md.ray_id());
                debug!(
                    event = "msg received",
                    src = ?md.src(),
                    kad_height = md.height(),
                    ray_id,
                    topic = ?msg.topic(),
                    height = msg.get_height(),
                    iteration = msg.get_iteration(),
                );

                // Update Transport Data
                msg.metadata = Some(Metadata {
                    height: md.height(),
                    src_addr: md.src(),
                    ray_id,
                });

                // Allow upper layers to fast-discard a message before queueing
                if let Err(e) = self.call_filters(msg.topic(), &msg) {
                    info!("discard message due to {e}");
                    return;
                }

                // Reroute message to the upper layer
                self.reroute(msg.topic().into(), msg);
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

    /// Represents a parsed conf.public_addr
    public_addr: SocketAddr,

    counter: AtomicU64,
}

impl<const N: usize> Kadcast<N> {
    pub fn new(mut conf: Config) -> Result<Self, AddrParseError> {
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
        };
        conf.version = format!("{PROTOCOL_VERSION}");
        conf.version_match = format!("{PROTOCOL_VERSION}");
        let peer = Peer::new(conf.clone(), listener)?;
        let public_addr = conf
            .public_address
            .parse::<SocketAddr>()
            .expect("valid kadcast public address");

        let nonce = Nonce::from(public_addr.ip());

        Ok(Kadcast {
            routes,
            filters,
            peer,
            conf,
            public_addr,
            counter: AtomicU64::new(nonce.into()),
        })
    }

    pub fn route_internal(&self, msg: Message) {
        let topic = msg.topic() as usize;
        let routes = self.routes.clone();

        tokio::spawn(async move {
            if let Some(Some(queue)) = routes.read().await.get(topic) {
                queue.try_send(msg.clone());
            };
        });
    }

    pub async fn alive_nodes(&self, amount: usize) -> Vec<SocketAddr> {
        self.peer.alive_nodes(amount).await
    }

    pub async fn table(&self) -> Vec<SocketAddr> {
        self.peer
            .to_route_table()
            .await
            .into_values()
            .flat_map(|v| v.into_iter().map(|(addr, _)| addr))
            .collect()
    }

    pub fn conf(&self) -> &Config {
        &self.conf
    }

    async fn send_with_metrics(
        &self,
        bytes: &Vec<u8>,
        recv_addr: Vec<SocketAddr>,
    ) {
        if !recv_addr.is_empty() {
            let bytes_sent = bytes.len() * recv_addr.len();
            counter!("dusk_bytes_sent").increment(bytes_sent as u64);
            self.peer.send_to_peers(bytes, recv_addr).await;
        }
    }
}

#[async_trait]
impl<const N: usize> crate::Network for Kadcast<N> {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        let kad_height = msg.metadata.as_ref().map(|m| m.height);
        debug!(
            event = "broadcasting msg",
            kad_height,
            ray_id = msg.ray_id(),
            topic = ?msg.topic(),
            height = msg.get_height(),
            iteration = msg.get_iteration(),
        );

        let height = match kad_height {
            Some(0) => return Ok(()),
            Some(height) => Some(height - 1),
            None => None,
        };

        let mut encoded = vec![];
        msg.write(&mut encoded).map_err(|err| {
            error!("could not encode message (version: {:?}, topics: {:?}, header: {:?}): {err}", msg.version(), msg.topic(), msg.header);
            anyhow::anyhow!("failed to broadcast: {err}")
        })?;

        counter!("dusk_bytes_cast").increment(encoded.len() as u64);
        counter!(format!("dusk_outbound_{:?}_size", msg.topic()))
            .increment(encoded.len() as u64);

        self.peer.broadcast(&encoded, height).await;

        Ok(())
    }

    /// Broadcast a GetResource request.
    ///
    /// By utilizing the randomly selected peers per bucket in Kadcast, this
    /// broadcast does follow the so-called "Flood with Random Walk" blind
    /// search (resource discovery).
    ///
    /// A receiver of this message is supposed to look up the resource and
    /// either return it or, if not found, rebroadcast the message to the next
    /// Kadcast bucket
    ///
    /// * `ttl_as_sec` - Defines the lifespan of the request in seconds
    ///
    /// * `hops_limit` - Defines maximum number of hops to receive the request
    async fn flood_request(
        &self,
        msg_inv: &Inv,
        ttl_as_sec: Option<u64>,
        hops_limit: u16,
    ) -> anyhow::Result<()> {
        let ttl_as_sec = ttl_as_sec
            .map_or_else(|| u64::MAX, |v| get_current_timestamp() + v);

        let msg = GetResource::new(
            msg_inv.clone(),
            Some(self.public_addr),
            ttl_as_sec,
            hops_limit,
        );
        self.send_to_alive_peers(msg.into(), REDUNDANCY_PEER_COUNT)
            .await
    }

    /// Sends an encoded message to a given peer.
    async fn send_to_peer(
        &self,
        mut msg: Message,
        recv_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        // rnd_count is added to bypass kadcast dupemap
        let rnd_count = self.counter.fetch_add(1, Ordering::SeqCst);

        msg.payload.set_nonce(rnd_count);

        let mut encoded = vec![];
        msg.write(&mut encoded)
            .map_err(|err| anyhow::anyhow!("failed to send_to_peer: {err}"))?;
        let topic = msg.topic();

        debug!(
          event = "Sending msg",
          topic = ?topic,
          info = ?msg.header,
          destination = ?recv_addr
        );

        self.send_with_metrics(&encoded, vec![recv_addr]).await;

        Ok(())
    }

    /// Sends to random set of alive peers.
    async fn send_to_alive_peers(
        &self,
        mut msg: Message,
        amount: usize,
    ) -> anyhow::Result<()> {
        // rnd_count is added to bypass kadcast dupemap
        let rnd_count = self.counter.fetch_add(1, Ordering::SeqCst);

        msg.payload.set_nonce(rnd_count);

        let mut encoded = vec![];
        msg.write(&mut encoded)
            .map_err(|err| anyhow::anyhow!("failed to encode: {err}"))?;
        let topic = msg.topic();

        counter!(format!("dusk_requests_{:?}", topic)).increment(1);

        let mut alive_nodes = self.peer.alive_nodes(amount).await;

        if alive_nodes.len() < amount {
            let current = alive_nodes.len();

            let route_table = self.peer.to_route_table().await;
            let new_nodes: Vec<_> = route_table
                .into_values()
                .flatten()
                .map(|(s, _)| s)
                .filter(|s| !alive_nodes.contains(s))
                .take(amount - current)
                .collect();

            alive_nodes.extend(new_nodes);
            warn!(
                event = "Not enought alive peers to send msg, increased",
                ?topic,
                requested = amount,
                current,
                increased = alive_nodes.len(),
            );
        }
        trace!("sending msg ({topic:?}) to peers {alive_nodes:?}");
        self.send_with_metrics(&encoded, alive_nodes).await;

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

    // TODO: Duplicated func
    fn get_info(&self) -> anyhow::Result<String> {
        Ok(self.conf.public_address.to_string())
    }

    fn public_addr(&self) -> &SocketAddr {
        &self.public_addr
    }

    async fn alive_nodes_count(&self) -> usize {
        // TODO: This call should be replaced with no-copy Kadcast API
        self.peer.alive_nodes(u16::MAX as usize).await.len()
    }
}
