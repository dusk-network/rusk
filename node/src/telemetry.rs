// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{database, vm, LongLivedService, Network};
use async_trait::async_trait;
use metrics::{describe_counter, describe_histogram, Unit};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct TelemetrySrv {}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for TelemetrySrv
{
    /// Assigns a description to any metric collected
    async fn initialize(
        &mut self,
        _network: Arc<RwLock<N>>,
        _db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<()> {
        describe_histogram!(
            "dusk_block_est_elapsed",
            "The elapsed time of EST call on block acceptance."
        );
        describe_counter!(
            "dusk_block_{Final,Accepted, Attested}",
            "The Cumulative number of all blocks by label."
        );

        describe_counter!(
            "dusk_outbound_Quorum_size",
            Unit::Bytes,
            "The Cumulative size of inbound messages by type."
        );

        // TODO: add it
        // TODO: consider other metrics
        // Slashed_all/Shashed_this/ Generator/ Provisioners / Bytes_sent/
        // Bytes_recv / Latency block
        describe_counter!(
            "dusk_fallbacks",
            "The Cumulative number of fallback execution."
        );

        Ok(())
    }

    /// Initialize and spawn Prometheus Exporter and Recorder
    /// By default, recorder exposes metrics to localhost:9000/metrics
    async fn execute(
        &mut self,
        _network: Arc<RwLock<N>>,
        _db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // TODO: Disable/Enable by Toml config

        // If PrometheusBuilder Recorder is not enabled then a NOOP
        // (No-overhead) Recorder is used by default.
        let (recorder, exporter) = PrometheusBuilder::new().build()?;
        metrics::set_global_recorder(recorder)?;
        exporter.await?;
        Ok(0)
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "telemetry"
    }
}
