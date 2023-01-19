// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{any, default, net::IpAddr, sync::Arc};

use crate::{utils::PendingQueue, BoxedFilter, Message};
use async_trait::async_trait;
use kadcast::{config::Config, MessageInfo, Peer};
use tokio::sync::RwLock;

mod frame;

type RoutesList<const N: usize> = [Option<PendingQueue<Message>>; N];
type FilterList<const N: usize> = [Option<BoxedFilter>; N];

pub struct Listener<const N: usize> {
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
}

impl<const N: usize> Listener<N> {
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

    fn call_filters(
        &self,
        topic: impl Into<u8>,
        msg: Message,
    ) -> anyhow::Result<()> {
        let topic = topic.into() as usize;

        match self.filters.try_write()?.get_mut(topic) {
            Some(Some(f)) => f.filter(&msg),
            _ => anyhow::Ok(()),
        }
    }
}

impl<const N: usize> kadcast::NetworkListen for Listener<N> {
    fn on_message(&self, message: Vec<u8>, md: MessageInfo) {
        // TODO: Decode message

        if let Err(e) = self.call_filters(0, Message::default()) {
            /// Discarding message
            tracing::trace!("discard message due to {:?}", e);
            return;
        }

        if let Err(e) = self.reroute(0, Message::default()) {
            tracing::error!("could not dispatch {:?}", e);
        }
    }
}

pub struct Kadcast<const N: usize> {
    peer: Peer,
    routes: Arc<RwLock<RoutesList<N>>>,
    filters: Arc<RwLock<FilterList<N>>>,
}

impl<const N: usize> Kadcast<N> {
    pub fn new(conf: Config) -> Self {
        const INIT: Option<PendingQueue<Message>> = None;
        let routes = Arc::new(RwLock::new([INIT; N]));

        const INIT_FN: Option<BoxedFilter> = None;
        let filters = Arc::new(RwLock::new([INIT_FN; N]));

        Kadcast {
            routes: routes.clone(),
            filters: filters.clone(),
            peer: Peer::new(conf, Listener { routes, filters }).unwrap(),
        }
    }
}

#[async_trait]
impl<const N: usize> crate::Network for Kadcast<N> {
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()> {
        // TODO: broadcast
        self.peer.broadcast(&[0u8; 8], None).await;

        anyhow::Ok(())
    }

    async fn repropagate(
        &self,
        msg: &Message,
        from_height: u8,
    ) -> anyhow::Result<()> {
        // TODO: repropagate message with this height
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
        let mut guard = self.routes.write().await;

        let mut route = guard
            .get_mut(msg_type as usize)
            .expect("should be a valid type");

        assert!(route.is_none(), "msg type already registered");

        *route = Some(queue);

        anyhow::Ok(())
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

        anyhow::Ok(())
    }
}
