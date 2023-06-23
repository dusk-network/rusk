// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
mod ephemeral;
mod version;
mod vm;

use std::path::PathBuf;

use clap::{Arg, Command};
use node::database::rocksdb;
use node::database::DB;
use node::LongLivedService;
use rusk::{Result, Rusk};
use rustc_tools_util::get_version_info;
use version::show_version;

use dusk_consensus::config::ACCUMULATOR_WORKERS_AMOUNT;
use node::chain::ChainSrv;
use node::databroker::DataBrokerSrv;
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::Node;

use crate::config::Config;

// #[tokio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_info = get_version_info!();
    let crate_name = &crate_info.crate_name.to_string();
    let version = show_version(crate_info);
    let command = Command::new(crate_name)
        .version(version.as_str())
        .author("Dusk Network B.V. All Rights Reserved.")
        .about("Rusk Server node.")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .env("RUSK_CONFIG_TOML")
                .help("Configuration file path")
                .takes_value(true)
                .required(false),
        );

    let command = ephemeral::inject_args(command);
    let command = Config::inject_args(command);
    let args = command.get_matches();
    let config = Config::from(&args);

    let log = config.log_level();

    // Generate a subscriber with the desired log level.
    let subscriber =
        tracing_subscriber::fmt::Subscriber::builder().with_max_level(log);

    // Set the subscriber as global.
    // so this subscriber will be used as the default in all threads for the
    // remainder of the duration of the program, similar to how `loggers`
    // work in the `log` crate.
    match config.log_type().as_str() {
        "json" => {
            let subscriber = subscriber.json().flatten_event(true).finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        "plain" => {
            let subscriber = subscriber.with_ansi(false).finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        "coloured" => {
            let subscriber = subscriber.finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        _ => unreachable!(),
    };

    let tempdir = match args.get_one::<PathBuf>("state_file") {
        Some(state_zip) => ephemeral::configure(state_zip)?,
        None => None,
    };
    let state_dir = rusk_profile::get_rusk_state_dir()?;
    tracing::info!("Using state from {state_dir:?}");
    let rusk = Rusk::new(state_dir)?;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2 + ACCUMULATOR_WORKERS_AMOUNT)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // Set up a node where:
            // transport layer is Kadcast with message ids from 0 to 255
            // persistence layer is rocksdb
            type Services = dyn LongLivedService<
                Kadcast<255>,
                rocksdb::Backend,
                vm::VMExecutionImpl,
            >;

            // Select list of services to enable
            let service_list: Vec<Box<Services>> = vec![
                Box::<MempoolSrv>::default(),
                Box::new(ChainSrv::new(config.chain.consensus_keys_path())),
                Box::new(DataBrokerSrv::new(config.databroker())),
            ];

            let db_path = tempdir.as_ref().map_or_else(
                || config.chain.db_path(),
                |t| t.path().to_path_buf(),
            );
            let db = rocksdb::Backend::create_or_open(db_path);
            let net = Kadcast::new(config.clone().kadcast.into());
            let vm =
                vm::VMExecutionImpl::new(node::vm::Config::default(), rusk);

            // node spawn_all is the entry point
            if let Err(e) = Node::new(net, db, vm).spawn_all(service_list).await
            {
                tracing::error!("node terminated with err: {}", e);
                Err(e)
            } else {
                Ok(())
            }
        })?;

    Ok(())
}
