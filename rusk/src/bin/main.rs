// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

mod args;
mod config;
#[cfg(feature = "ephemeral")]
mod ephemeral;
mod log;

use clap::Parser;

use log::Log;

#[cfg(feature = "node")]
use rusk::node::{Rusk, RuskNodeBuilder};

use rusk::http::{DataSources, HttpServer};
use rusk::Result;

use tokio::sync::broadcast;

use tracing::info;

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

    let channel_cap = config.http.ws_event_channel_cap;
    let (_event_sender, event_receiver) = broadcast::channel(channel_cap);

    #[cfg(feature = "node")]
    let mut node_builder = {
        let state_dir = rusk_profile::get_rusk_state_dir()?;
        info!("Using state from {state_dir:?}");

        let rusk = Rusk::new(
            state_dir,
            config.kadcast.chain_id(),
            config.chain.generation_timeout(),
            config.chain.charge_per_deploy_byte(),
            config.chain.block_gas_limit(),
            config.http.feeder_call_gas,
            _event_sender.clone(),
        )?;

        info!("Rusk VM loaded");

        #[cfg(feature = "ephemeral")]
        let db_path = tempdir.as_ref().map_or_else(
            || config.chain.db_path(),
            |t| std::path::Path::to_path_buf(t.path()),
        );

        #[cfg(not(feature = "ephemeral"))]
        let db_path = config.chain.db_path();

        RuskNodeBuilder::new(rusk)
            .with_db_path(db_path)
            .with_db_options(config.chain.db_options())
            .with_kadcast(config.kadcast)
            .with_consensus_keys(config.chain.consensus_keys_path())
            .with_databroker(config.databroker)
            .with_telemetry(config.telemetry.listen_addr())
            .with_chain_queue_size(config.chain.max_queue_size())
            .with_mempool(config.mempool.into())
            .with_rues(_event_sender)
    };
    let mut _ws_server = None;
    if config.http.listen {
        info!("Configuring HTTP");

        #[allow(unused_mut)]
        let mut handler = DataSources::default();

        #[cfg(feature = "prover")]
        handler.sources.push(Box::new(rusk_prover::LocalProver));

        #[cfg(feature = "node")]
        handler.sources.extend(node_builder.build_data_sources()?);

        let listen_addr = config.http.listen_addr();

        let cert_and_key = match (config.http.cert, config.http.key) {
            (Some(cert), Some(key)) => Some((cert, key)),
            _ => None,
        };

        let ws_event_channel_cap = config.http.ws_event_channel_cap;

        _ws_server = Some(
            HttpServer::bind(
                handler,
                event_receiver,
                ws_event_channel_cap,
                listen_addr,
                cert_and_key,
            )
            .await?,
        );
    }

    #[cfg(feature = "node")]
    if let Err(e) = node_builder.build_and_run().await {
        tracing::error!("node terminated with err: {}", e);
        return Err(e.into());
    }

    #[cfg(not(feature = "node"))]
    if let Some(s) = _ws_server {
        s.handle.await?;
    }

    Ok(())
}
