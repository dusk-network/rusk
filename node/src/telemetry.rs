// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{database, vm, LongLivedService, Network};
use async_trait::async_trait;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        _: Arc<RwLock<N>>,
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
            exporter.await?;
        }
        Ok(0)
    }
}

impl TelemetrySrv {
    pub fn new(addr: Option<String>) -> Self {
        Self { addr }
    }
}
