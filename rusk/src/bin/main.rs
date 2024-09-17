// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod args;
mod config;
#[cfg(feature = "ephemeral")]
mod ephemeral;
mod log;

use clap::Parser;

use log::Log;

use rusk::Builder;

use rusk::http::HttpServerConfig;
use rusk::Result;

use crate::config::Config;

// Number of workers should be at least `ACCUMULATOR_WORKERS_AMOUNT` from
// `dusk_consensus::config`.
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::Args::parse();

    let config = Config::from(&args);

    let log = Log::new(config.log_level(), config.log_filter());

    #[cfg(any(feature = "recovery-state", feature = "recovery-keys"))]
    // Set custom tracing format if subcommand is specified
    if let Some(command) = args.command {
        log.register()?;
        command.run()?;
        return Ok(());
    }

    log.with_format(config.log_type()).register()?;

    #[cfg(feature = "ephemeral")]
    let tempdir = match args.state_path {
        Some(state_zip) => ephemeral::configure(&state_zip)?,
        None => None,
    };

    let mut node_builder = Builder::default();

    #[cfg(feature = "chain")]
    {
        let state_dir = rusk_profile::get_rusk_state_dir()?;

        #[cfg(feature = "ephemeral")]
        let db_path = tempdir.as_ref().map_or_else(
            || config.chain.db_path(),
            |t| std::path::Path::to_path_buf(t.path()),
        );

        #[cfg(not(feature = "ephemeral"))]
        let db_path = config.chain.db_path();

        node_builder = node_builder
            .with_feeder_call_gas(config.http.feeder_call_gas)
            .with_db_path(db_path)
            .with_db_options(config.chain.db_options())
            .with_kadcast(config.kadcast)
            .with_consensus_keys(config.chain.consensus_keys_path())
            .with_databroker(config.databroker)
            .with_telemetry(config.telemetry.listen_addr())
            .with_chain_queue_size(config.chain.max_queue_size())
            .with_mempool(config.mempool.into())
            .with_state_dir(state_dir)
            .with_generation_timeout(config.chain.generation_timeout())
            .with_gas_per_deploy_byte(config.chain.gas_per_deploy_byte())
            .with_min_deployment_gas_price(
                config.chain.min_deployment_gas_price(),
            )
            .with_block_gas_limit(config.chain.block_gas_limit())
    }

    if config.http.listen {
        let http_builder = HttpServerConfig {
            address: config.http.listen_addr(),
            cert: config.http.cert,
            key: config.http.key,
            ws_event_channel_cap: config.http.ws_event_channel_cap,
        };
        node_builder = node_builder.with_http(http_builder)
    }

    if let Err(e) = node_builder.build_and_run().await {
        tracing::error!("node terminated with err: {}", e);
        return Err(e.into());
    }

    Ok(())
}
