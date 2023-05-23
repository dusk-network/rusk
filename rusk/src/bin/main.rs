// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
mod ephemeral;
mod services;
#[cfg(not(target_os = "windows"))]
mod unix;
mod version;

use std::path::PathBuf;

use clap::{Arg, Command};
use rusk::services::network::KadcastDispatcher;
use rusk::services::network::NetworkServer;
use rusk::services::prover::{ProverServer, RuskProver};
use rusk::services::state::StateServer;
use rusk::services::version::{CompatibilityInterceptor, RuskVersionLayer};
use rusk::{Result, Rusk};
use rustc_tools_util::get_version_info;
use tonic::transport::Server;
use version::show_version;

use services::startup_with_tcp_ip;
use services::startup_with_uds;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let _tempdir = match args.get_one::<PathBuf>("state_zip_file") {
        Some(state_zip) => ephemeral::configure(state_zip)?,
        None => None,
    };

    let router = {
        let rusk = Rusk::new(rusk_profile::get_rusk_state_dir()?)?;

        let kadcast = KadcastDispatcher::new(
            config.kadcast.clone().into(),
            config.kadcast_test,
        )?;

        let network =
            NetworkServer::with_interceptor(kadcast, CompatibilityInterceptor);
        let state =
            StateServer::with_interceptor(rusk, CompatibilityInterceptor);
        let prover = ProverServer::with_interceptor(
            RuskProver::default(),
            CompatibilityInterceptor,
        );

        Server::builder()
            .layer(RuskVersionLayer)
            .add_service(network)
            .add_service(state)
            .add_service(prover)
    };

    // Match the desired IPC method. Or set the default one depending on the OS
    // used. Then startup rusk with the final values.
    match config.grpc.ipc_method.as_deref() {
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
    }
}
