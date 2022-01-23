// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
mod services;
#[cfg(not(target_os = "windows"))]
mod unix;
mod version;

use clap::{App, Arg};
use rusk::services::network::KadcastDispatcher;
use rusk::services::network::NetworkServer;
use rusk::services::pki::KeysServer;
use rusk::services::prover::ProverServer;
use rusk::services::state::StateServer;
use rusk::Rusk;
use rustc_tools_util::{get_version_info, VersionInfo};
use tonic::transport::Server;
use version::show_version;

use services::startup_with_tcp_ip;
use services::startup_with_uds;

use crate::config::Config;

#[tokio::main]
async fn main() {
    let crate_info = get_version_info!();
    let crate_name = &crate_info.crate_name.to_string();
    let version = show_version(crate_info);
    let app = App::new(crate_name)
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
        )
        .arg(
            Arg::new("socket")
                .short('s')
                .long("socket")
                .value_name("socket")
                .help("Path for setting up the UDS ")
                .takes_value(true),
        )
        .arg(
            Arg::new("ipc_method")
                .long("ipc_method")
                .value_name("ipc_method")
                .possible_values(&["uds", "tcp_ip"])
                .help("Inter-Process communication protocol you want to use ")
                .takes_value(true),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("port")
                .help("Port you want to use ")
                .takes_value(true),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("host")
                .takes_value(true),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LOG")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .help("Output log level")
                .takes_value(true),
        );
    let app = network_config(app);
    let cfg = Config::from(app.get_matches());

    // Match tracing desired level.
    let log = match &cfg.log_level[..] {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => unreachable!(),
    };

    // Generate a subscriber with the desired log level.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(log)
        .finish();
    // Set the subscriber as global.
    // so this subscriber will be used as the default in all threads for the
    // remainder of the duration of the program, similar to how `loggers`
    // work in the `log` crate.
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed on subscribe tracing");

    let service = {
        let kadcast =
            KadcastDispatcher::new(cfg.kadcast.clone(), cfg.kadcast_test);

        let network = NetworkServer::new(kadcast);
        let rusk = Rusk::default();
        let keys = KeysServer::new(rusk);
        let state = StateServer::new(rusk);
        let prover = ProverServer::new(rusk);

        Server::builder()
            .add_service(keys)
            .add_service(network)
            .add_service(state)
            .add_service(prover)
    };

    // Match the desired IPC method. Or set the default one depending on the OS
    // used. Then startup rusk with the final values.
    let res = match cfg.ipc_method.as_deref() {
        Some(method) => match (cfg!(windows), method) {
            (_, "tcp_ip") => {
                startup_with_tcp_ip(&cfg.host, &cfg.port, service).await
            }
            (true, "uds") => {
                panic!("Windows does not support Unix Domain Sockets");
            }
            (false, "uds") => startup_with_uds(&cfg.socket, service).await,
            (_, _) => unreachable!(),
        },
        None => {
            if cfg!(windows) {
                startup_with_tcp_ip(&cfg.host, &cfg.port, service).await
            } else {
                startup_with_uds(&cfg.socket, service).await
            }
        }
    };
    match res {
        Ok(()) => (),
        Err(e) => eprintln!("{}", e),
    };
}

/// Setup clap to handle kadcast network configuration
fn network_config(app: App<'_>) -> App<'_> {
    app.arg(
        Arg::new("kadcast_public_address")
            .long("kadcast_public_address")
            .long_help("This is the address where other peer can contact you. 
This address MUST be accessible from any peer of the network")
            .help("Public address you want to be identified with. Eg: 193.xxx.xxx.198:9999")
            .env("KADCAST_PUBLIC_ADDRESS")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::new("kadcast_listen_address")
            .long("kadcast_listen_address")
            .long_help("This address is the one bound for the incoming connections. 
Use this argument if your host is not publicly reachable from other peer in the network 
(Eg: if you are behind a NAT)
If this is not specified, the public address will be used for binding incoming connection")
            .help("Optional internal address to listen incoming connections. Eg: 127.0.0.1:9999")
            .env("KADCAST_LISTEN_ADDRESS")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::new("kadcast_bootstrap")
            .long("kadcast_bootstrap")
            .env("KADCAST_BOOTSTRAP")
            .multiple_occurrences(true)
            .help("Kadcast list of bootstrapping server addresses")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::new("kadcast_autobroadcast")
            .long("kadcast_autobroadcast")
            .env("KADCAST_AUTOBROADCAST")
            .help("If used then the received messages are automatically re-broadcasted")
            .takes_value(false)
            .required(false),
    )
    .arg(
        Arg::new("kadcast_test")
            .long("kadcast_test")
            .env("KADCAST_TEST")
            .help("If used then the received messages is a blake2b 256hash")
            .takes_value(false)
            .required(false),
    )
}
