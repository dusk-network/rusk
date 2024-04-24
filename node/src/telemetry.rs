// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{database, vm, LongLivedService, Network};
use async_trait::async_trait;
use memory_stats::memory_stats;
use metrics::histogram;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Default)]
pub struct TelemetrySrv {
    addr: Option<String>,
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for TelemetrySrv
{
    /// Returns service name.
    fn name(&self) -> &'static str {
        "telemetry"
    }

    async fn initialize(
        &mut self,
        _network: Arc<RwLock<N>>,
        _db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Initialize and spawn Prometheus Exporter and Recorder
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        _: Arc<RwLock<DB>>,
        _: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // If PrometheusBuilder Recorder is not enabled then a NOOP
        // (No-overhead) recorder is used by default.
        if let Some(addr) = &self.addr {
            let addr = addr.parse::<SocketAddr>()?;
            let (recorder, exporter) =
                PrometheusBuilder::new().with_http_listener(addr).build()?;
            metrics::set_global_recorder(recorder)?;
            tokio::spawn(exporter);

            loop {
                sleep(Duration::from_secs(5)).await;
                // Record memory stats
                if let Some(usage) = memory_stats() {
                    histogram!("dusk_physical_mem")
                        .record(usage.physical_mem as f64);
                    histogram!("dusk_virtual_mem")
                        .record(usage.virtual_mem as f64);
                }

                // Record number of alive kadcast peers
                let count = network.read().await.alive_nodes_count().await;
                histogram!("dusk_kadcast_peers").record(count as f64);
            }
        }
        Ok(0)
    }
}

impl TelemetrySrv {
    pub fn new(addr: Option<String>) -> Self {
        Self { addr }
    }
}
