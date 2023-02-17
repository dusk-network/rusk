// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::chain::ChainSrv;
use node::database::{rocksdb, DB};
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::{LongLivedService, Node};

use clap::{Arg, ArgMatches, Command};
use rustc_tools_util::{get_version_info, VersionInfo};
use version::show_version;

use crate::config::Config;

mod config;
mod version;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = args();
    let config = Config::from(&args);

    configure_log(&config)?;

    // Set up a node where:
    // transport layer is Kadcast with message ids from 0 to 255
    // persistence layer is rocksdb
    type Services = dyn LongLivedService<Kadcast<255>, rocksdb::Backend>;

    // Select list of services to enable
    let service_list: Vec<Box<Services>> =
        vec![Box::<MempoolSrv>::default(), Box::<ChainSrv>::default()];

    let db = rocksdb::Backend::create_or_open("".to_string());
    let net = Kadcast::new(config.network.into());

    // node spawn_all is the entry point
    if let Err(e) = Node::new(net, db).spawn_all(service_list).await {
        tracing::error!("node terminated with err: {}", e);
        Err(e)
    } else {
        Ok(())
    }
}

fn args() -> ArgMatches {
    let crate_info = get_version_info!();
    let crate_name = &crate_info.crate_name.to_string();
    let version = show_version(crate_info);
    let command = Command::new(crate_name)
        .version(version.as_str())
        .author("Dusk Network B.V. All Rights Reserved.")
        .about("Dusk Server node.")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .env("DUSK_CONFIG_TOML")
                .help("Configuration file path")
                .takes_value(true)
                .required(false),
        );

    let command = Config::inject_args(command);
    command.get_matches()
}

fn configure_log(config: &Config) -> anyhow::Result<()> {
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
    Ok(())
}
