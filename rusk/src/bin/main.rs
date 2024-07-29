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
use node::{
    chain::ChainSrv,
    database::{rocksdb, DB},
    databroker::DataBrokerSrv,
    mempool::MempoolSrv,
    network::Kadcast,
    telemetry::TelemetrySrv,
    LongLivedService, Node,
};
use rusk::http::{DataSources, HttpServer};
#[cfg(feature = "node")]
use rusk::node::Rusk;
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
    let (rusk, node, mut service_list) = {
        let state_dir = rusk_profile::get_rusk_state_dir()?;
        info!("Using state from {state_dir:?}");

        let rusk = Rusk::new(
            state_dir,
            config.chain.generation_timeout(),
            config.chain.gas_per_deploy_byte(),
            config.http.feeder_call_gas,
            _event_sender,
        )?;

        info!("Rusk VM loaded");

        // Set up a node where:
        // transport layer is Kadcast with message ids from 0 to 255
        // persistence layer is rocksdb
        type Services =
            dyn LongLivedService<Kadcast<255>, rocksdb::Backend, Rusk>;

        // Select list of services to enable
        let service_list: Vec<Box<Services>> = vec![
            Box::<MempoolSrv>::default(),
            Box::new(ChainSrv::new(config.chain.consensus_keys_path())),
            Box::new(DataBrokerSrv::new(config.clone().databroker.into())),
            Box::new(TelemetrySrv::new(config.telemetry.listen_addr())),
        ];

        #[cfg(feature = "ephemeral")]
        let db_path = tempdir.as_ref().map_or_else(
            || config.chain.db_path(),
            |t| std::path::Path::to_path_buf(t.path()),
        );

        #[cfg(not(feature = "ephemeral"))]
        let db_path = config.chain.db_path();

        let db = rocksdb::Backend::create_or_open(
            db_path,
            config.chain.db_options(),
        );
        let net = Kadcast::new(config.clone().kadcast.into())?;

        let node = rusk::node::RuskNode(Node::new(net, db, rusk.clone()));
        (rusk, node, service_list)
    };
    let mut _ws_server = None;
    if config.http.listen {
        info!("Configuring HTTP");

        let handler = DataSources {
            #[cfg(feature = "node")]
            node: node.clone(),
            #[cfg(feature = "node")]
            rusk,
            #[cfg(feature = "prover")]
            prover: rusk_prover::LocalProver,
        };

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
    // initialize all registered services
    if let Err(err) = node.0.initialize(&mut service_list).await {
        tracing::error!("node initialization failed: {err}");
        return Err(err.into());
    }

    #[cfg(feature = "node")]
    // node spawn_all is the entry point
    if let Err(e) = node.0.spawn_all(service_list).await {
        tracing::error!("node terminated with err: {}", e);
        return Err(e.into());
    }

    #[cfg(not(feature = "node"))]
    if let Some(s) = _ws_server {
        s.handle.await?;
    }

    Ok(())
}
