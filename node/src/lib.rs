// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
#![allow(unused)]

pub mod chain;
mod data;
pub mod mempool;
pub mod network;
mod utils;

use crate::utils::PendingQueue;
use async_trait::async_trait;
use data::Topics;
use std::sync::Arc;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::RwLock,
    task::JoinSet,
};
use tracing::{error, info, Instrument};

#[derive(Clone, Default)]
pub struct Message {
    topic: Topics,
}

#[async_trait]
pub trait Network: Send + Sync + 'static {
    /// Broadcasts a message.
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()>;

    /// Repropagates a received message.
    async fn repropagate(
        &self,
        msg: &Message,
        from_height: u8,
    ) -> anyhow::Result<()>;

    /// Sends a message to specified peers.
    async fn send(&self, msg: &Message, dst: Vec<String>)
        -> anyhow::Result<()>;

    /// Routes any message of the specified type to this queue.
    async fn add_route(
        &mut self,
        msg_type: u8,
        queue: PendingQueue<Message>,
    ) -> anyhow::Result<()>;
}

/// Service processes specified set of messages and eventually produces a
/// DataSource query or update.
///
/// Service is allowed to propagate a message to the network as well.
#[async_trait]
pub trait LongLivedService<N: Network>: Send + Sync {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
    ) -> anyhow::Result<usize>;

    async fn add_routes(
        &self,
        my_topics: &[u8],
        queue: PendingQueue<Message>,
        network: &Arc<RwLock<N>>,
    ) -> anyhow::Result<()> {
        let mut guard = network.write().await;
        for topic in my_topics {
            guard.add_route(*topic, queue.clone()).await?
        }
        anyhow::Ok(())
    }

    /// Returns service name.
    fn name(&self) -> &'static str;
}

pub struct Node<N: Network> {
    network: Arc<RwLock<N>>,
    // TODO: data_source: Arc<DataSource>,
}

impl<N: Network> Node<N> {
    pub fn new(n: N) -> Self {
        Self {
            network: Arc::new(RwLock::new(n)),
        }
    }

    /// Sets up and runs a list of services.
    pub async fn spawn_all(
        &self,
        service_list: Vec<Box<dyn LongLivedService<N>>>,
    ) -> anyhow::Result<()> {
        // Initialize DataSources
        // TODO:

        // Initialize Rusk instance
        // TODO:

        // Spawn all services and join-wait for their termination.
        let mut set = JoinSet::new();
        set.spawn(async {
            signal(SignalKind::interrupt())?.recv().await;
            // TODO: ResultCode
            Ok(2)
        });

        for (mut s) in service_list.into_iter() {
            //let ds = self.data_source.clone();
            let n = self.network.clone();
            let name = s.name();

            info!("starting service {}", name);

            set.spawn(async move {
                s.execute(n)
                    .instrument(tracing::info_span!("srv", name))
                    .await
            });
        }

        // Wait for all spawned services to terminate with a result code or
        // an error. Result code 1 means abort all services.
        // This is usually triggered by SIGINIT signal.
        while let Some(res) = set.join_next().await {
            if let Ok(r) = res {
                match r {
                    Ok(rcode) => {
                        // handle SIGTERM signal
                        if rcode == 2 {
                            set.abort_all();
                        }
                    }
                    Err(e) => {
                        error!("service terminated with err{}", e);
                    }
                }
            }
        }

        info!("shutdown ...");

        // Release DataSource

        Ok(())
    }
}

pub fn enable_log(filter: impl Into<tracing::metadata::LevelFilter>) {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed on subscribe tracing");
}

#[cfg(test)]
mod tests {
    use super::*;
}
