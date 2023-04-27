// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

pub mod chain;
pub mod database;
pub mod mempool;
pub mod network;
pub mod vm;

use async_trait::async_trait;
use node_data::message::AsyncQueue;
use node_data::message::Message;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{error, info, Instrument};

/// Filter is used by Network implementor to filter messages before re-routing
/// them. It's like the middleware in HTTP pipeline.
///
/// To avoid delaying other messages handling, the execution of any filter
/// should be fast as it is performed in the message handler .
pub trait Filter {
    /// Filters a message.
    fn filter(&mut self, msg: &Message) -> anyhow::Result<()>;
}

pub type BoxedFilter = Box<dyn Filter + Sync + Send>;

#[async_trait]
pub trait Network: Send + Sync + 'static {
    /// Broadcasts a message.
    async fn broadcast(&self, msg: &Message) -> anyhow::Result<()>;

    /// Sends a message to specified peers.
    async fn send(&self, msg: &Message, dst: Vec<String>)
        -> anyhow::Result<()>;

    /// Routes any message of the specified type to this queue.
    async fn add_route(
        &mut self,
        msg_type: u8,
        queue: AsyncQueue<Message>,
    ) -> anyhow::Result<()>;

    /// Moves a filter of a specified topic to Network.
    async fn add_filter(
        &mut self,
        msg_type: u8,
        filter: BoxedFilter,
    ) -> anyhow::Result<()>;

    /// Retrieves information about the network.
    fn get_info(&self) -> anyhow::Result<String>;
}

/// Service processes specified set of messages and eventually produces a
/// DataSource query or update.
///
/// Service is allowed to propagate a message to the network as well.
#[async_trait]
pub trait LongLivedService<N: Network, DB: database::DB, VM: vm::VMExecution>:
    Send + Sync
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        database: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize>;

    async fn add_routes(
        &self,
        my_topics: &[u8],
        queue: AsyncQueue<Message>,
        network: &Arc<RwLock<N>>,
    ) -> anyhow::Result<()> {
        let mut guard = network.write().await;
        for topic in my_topics {
            guard.add_route(*topic, queue.clone()).await?
        }
        Ok(())
    }

    async fn add_filter(
        &self,
        topic: u8,
        filter_fn: BoxedFilter,
        network: &Arc<RwLock<N>>,
    ) -> anyhow::Result<()> {
        network.write().await.add_filter(topic, filter_fn).await?;
        Ok(())
    }

    /// Returns service name.
    fn name(&self) -> &'static str;
}

pub struct Node<N: Network, DB: database::DB, VM: vm::VMExecution> {
    network: Arc<RwLock<N>>,
    database: Arc<RwLock<DB>>,
    vm_handler: Arc<RwLock<VM>>,
}

impl<N: Network, DB: database::DB, VM: vm::VMExecution> Node<N, DB, VM> {
    pub fn new(n: N, d: DB, vm_h: VM) -> Self {
        Self {
            network: Arc::new(RwLock::new(n)),
            database: Arc::new(RwLock::new(d)),
            vm_handler: Arc::new(RwLock::new(vm_h)),
        }
    }

    /// Sets up and runs a list of services.
    pub async fn spawn_all(
        &self,
        service_list: Vec<Box<dyn LongLivedService<N, DB, VM>>>,
    ) -> anyhow::Result<()> {
        // Spawn all services and join-wait for their termination.
        let mut set = JoinSet::new();
        set.spawn(async {
            signal(SignalKind::interrupt())?.recv().await;
            // TODO: ResultCode
            Ok(2)
        });

        for (mut s) in service_list.into_iter() {
            let n = self.network.clone();
            let d = self.database.clone();
            let vm = self.vm_handler.clone();

            let name = s.name();
            info!("starting service {}", name);

            set.spawn(async move {
                s.execute(n, d, vm)
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

#[cfg(test)]
mod tests {
    use super::*;
}
