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

#[cfg(feature = "chain")]
use tracing::{info, warn};

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
    if let Some(args::command::Command::Recovery(recovery)) =
        args.command.clone()
    {
        // Set custom tracing format if subcommand is specified
        log.register()?;
        recovery.run()?;
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
        info!("Using state from {state_dir:?}");

        #[cfg(feature = "ephemeral")]
        let db_path = tempdir.as_ref().map_or_else(
            || config.chain.db_path(),
            |t| std::path::Path::to_path_buf(t.path()),
        );

        #[cfg(not(feature = "ephemeral"))]
        let db_path = config.chain.db_path();

        node_builder = node_builder
            .with_vm_config(config.vm)
            .with_feeder_call_gas(config.http.feeder_call_gas)
            .with_db_path(db_path)
            .with_db_options(config.chain.db_options())
            .with_kadcast(config.kadcast)
            .with_consensus_keys(config.chain.consensus_keys_path())
            .with_databroker(config.databroker)
            .with_telemetry(config.telemetry.listen_addr())
            .with_chain_queue_size(config.chain.max_queue_size())
            .with_genesis_timestamp(config.chain.genesis_timestamp())
            .with_mempool(config.mempool.into())
            .with_state_dir(state_dir)
            .with_blob_expire_after(config.blob.expire_after)
            .with_min_gas_limit(config.chain.min_gas_limit());

        #[allow(deprecated)]
        {
            if let Some(gas_byte) = config.chain.gas_per_deploy_byte() {
                warn!("[chain].gas_per_deploy_byte is deprecated, use [vm].gas_per_deploy_byte");
                node_builder = node_builder.with_gas_per_deploy_byte(gas_byte);
            }
            if let Some(price) = config.chain.min_deployment_gas_price() {
                warn!("[chain].min_deployment_gas_price is deprecated, use [vm].min_deployment_gas_price");
                node_builder =
                    node_builder.with_min_deployment_gas_price(price);
            }
            if let Some(timeout) = config.chain.generation_timeout() {
                warn!("[chain].generation_timeout is deprecated, use [vm].generation_timeout");
                node_builder = node_builder.with_generation_timeout(timeout);
            }
            if let Some(min) = config.chain.min_deploy_points() {
                warn!("[chain].min_deploy_points is deprecated, use [vm].min_deploy_points");
                node_builder = node_builder.with_min_deploy_points(min);
            }
            if let Some(limit) = config.chain.block_gas_limit() {
                warn!("[chain].block_gas_limit is deprecated, use [vm].block_gas_limit");
                node_builder = node_builder.with_block_gas_limit(limit);
            }
        }
    };

    if config.http.listen {
        let http_builder = HttpServerConfig {
            address: config.http.listen_addr(),
            cert: config.http.cert,
            key: config.http.key,
            headers: config.http.headers,
            ws_event_channel_cap: config.http.ws_event_channel_cap,
        };
        node_builder = node_builder.with_http(http_builder)
    }

    #[cfg(feature = "chain")]
    if let Some(args::command::Command::Chain(
        args::command::chain::ChainCommand::Revert,
    )) = args.command.as_ref()
    {
        node_builder = node_builder.with_revert();
    }

    if let Err(e) = node_builder.build_and_run().await {
        tracing::error!("node terminated with err: {}", e);
        return Err(e.into());
    }

    Ok(())
}
