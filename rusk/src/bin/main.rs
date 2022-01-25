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
use rusk::services::pki::{KeysServer, RuskKeys};
use rusk::services::prover::{ProverServer, RuskProver};
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
        );
    let app = Config::inject_args(app);
    let config = Config::from(app.get_matches());

    // Match tracing desired level.
    let log = match &config.log_level[..] {
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

    let router = {
        let rusk = Rusk::new().unwrap();
        let kadcast = KadcastDispatcher::new(
            config.kadcast.clone().into(),
            config.kadcast_test,
        );

        let keys = KeysServer::new(RuskKeys::default());
        let network = NetworkServer::new(kadcast);
        let state = StateServer::new(rusk);
        let prover = ProverServer::new(RuskProver::default());

        Server::builder()
            .add_service(keys)
            .add_service(network)
            .add_service(state)
            .add_service(prover)
    };

    // Match the desired IPC method. Or set the default one depending on the OS
    // used. Then startup rusk with the final values.
    let res = match config.grpc.ipc_method.as_deref() {
        Some(method) => match (cfg!(windows), method) {
            (_, "tcp_ip") => {
                startup_with_tcp_ip(
                    router,
                    &config.grpc.host,
                    &config.grpc.port,
                )
                .await
            }
            (true, "uds") => {
                panic!("Windows does not support Unix Domain Sockets");
            }
            (false, "uds") => {
                startup_with_uds(router, &config.grpc.socket).await
            }
            (_, _) => unreachable!(),
        },
        None => {
            if cfg!(windows) {
                startup_with_tcp_ip(
                    router,
                    &config.grpc.host,
                    &config.grpc.port,
                )
                .await
            } else {
                startup_with_uds(router, &config.grpc.socket).await
            }
        }
    };
    match res {
        Ok(()) => (),
        Err(e) => eprintln!("{}", e),
    };
}
