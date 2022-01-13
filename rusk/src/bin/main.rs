// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(not(target_os = "windows"))]
mod unix;
mod version;

use clap::{App, Arg, ArgMatches};
use futures::TryFutureExt;
use rusk::services::network::NetworkServer;
use rusk::services::network::RuskNetwork;
use rusk::services::pki::KeysServer;
use rusk::services::state::StateServer;
use rusk::Rusk;
use rustc_tools_util::{get_version_info, VersionInfo};
use std::path::Path;
use tokio::net::UnixListener;
use tonic::transport::Server;
use version::show_version;

/// Default UDS path that Rusk GRPC-server will connect to.
pub const SOCKET_PATH: &str = "/tmp/rusk_listener";

/// Default port that Rusk GRPC-server will listen to.
pub(crate) const PORT: &str = "8585";
/// Default host_address that Rusk GRPC-server will listen to.
pub(crate) const HOST_ADDRESS: &str = "127.0.0.1";

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
            Arg::with_name("socket")
                .short("s")
                .long("socket")
                .value_name("socket")
                .help("Path for setting up the UDS ")
                .default_value(SOCKET_PATH)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ipc_method")
                .long("ipc_method")
                .value_name("ipc_method")
                .possible_values(&["uds", "tcp_ip"])
                .help("Inter-Process communication protocol you want to use ")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("port")
                .help("Port you want to use ")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .short("h")
                .long("host")
                .value_name("host")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("log-level")
                .long("log-level")
                .value_name("LOG")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .default_value("info")
                .help("Output log level")
                .takes_value(true),
        );
    let app = network_config(app);
    let matches = app.get_matches();

    // Match tracing desired level.
    let log = match matches
        .value_of("log-level")
        .expect("Failed parsing log-level arg")
    {
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

    let network = create_network(&matches);

    // Match the desired IPC method. Or set the default one depending on the OS
    // used. Then startup rusk with the final values.
    let res = match matches.value_of("ipc_method") {
        Some(method) => match (cfg!(windows), method) {
            (_, "tcp_ip") => {
                startup_with_tcp_ip(
                    matches.value_of("host").unwrap_or(HOST_ADDRESS),
                    matches.value_of("port").unwrap_or(PORT),
                    network,
                )
                .await
            }
            (true, "uds") => {
                panic!("Windows does not support Unix Domain Sockets");
            }
            (false, "uds") => {
                startup_with_uds(
                    matches.value_of("socket").unwrap_or(SOCKET_PATH),
                    network,
                )
                .await
            }
            (_, _) => unreachable!(),
        },
        None => {
            if cfg!(windows) {
                startup_with_tcp_ip(
                    matches.value_of("host").unwrap_or(HOST_ADDRESS),
                    matches.value_of("port").unwrap_or(PORT),
                    network,
                )
                .await
            } else {
                startup_with_uds(
                    matches.value_of("socket").unwrap_or(SOCKET_PATH),
                    network,
                )
                .await
            }
        }
    };
    match res {
        Ok(()) => (),
        Err(e) => eprintln!("{}", e),
    };
}

#[cfg(not(target_os = "windows"))]
async fn startup_with_uds(
    path: &str,
    kadcast: RuskNetwork,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(Path::new(path).parent().unwrap()).await?;

    let uds = UnixListener::bind(path)?;

    let rusk = Rusk::default();

    let keys = KeysServer::new(rusk);
    let network = NetworkServer::new(kadcast);
    let state = StateServer::new(rusk);

    let incoming = {
        async_stream::stream! {
            loop {
                yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
            }
        }
    };

    Server::builder()
        .add_service(keys)
        .add_service(network)
        .add_service(state)
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}

async fn startup_with_tcp_ip(
    host: &str,
    port: &str,
    kadcast: RuskNetwork,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = host.to_string();
    full_address.push(':');
    full_address.push_str(&port.to_string());
    let addr: std::net::SocketAddr = full_address.parse()?;

    let rusk = Rusk::default();

    let keys = KeysServer::new(rusk);
    let network = NetworkServer::new(kadcast);
    let state = StateServer::new(rusk);

    // Build the Server with the `Echo` service attached to it.
    Ok(Server::builder()
        .add_service(keys)
        .add_service(network)
        .add_service(state)
        .serve(addr)
        .await?)
}

/// Setup clap to handle kadcast network configuration
fn network_config<'a>(app: App<'a, 'a>) -> App<'a, 'a> {
    app.arg(
        Arg::with_name("kadcast_public_address")
            .long("kadcast_public_address")
            .long_help("This is the address where other peer can contact you. 
This address MUST be accessible from any peer of the network")
            .help("Public address you want to be identified with. Eg: 193.xxx.xxx.198:9999")
            .env("KADCAST_PUBLIC_ADDRESS")
            .takes_value(true)
            .required(true),
    )
    .arg(
        Arg::with_name("kadcast_listen_address")
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
        Arg::with_name("kadcast_bootstrap")
            .long("kadcast_bootstrap")
            .env("KADCAST_BOOTSTRAP")
            .multiple(true)
            .help("Kadcast list of bootstrapping server addresses")
            .takes_value(true)
            .required(true),
    )
    .arg(
        Arg::with_name("kadcast_autobroadcast")
            .long("kadcast_autobroadcast")
            .env("KADCAST_AUTOBROADCAST")
            .help("If true then the received messages are automatically re-broadcasted")
            .takes_value(false)
            .required(false),
    )
}

fn create_network(args: &ArgMatches) -> RuskNetwork {
    RuskNetwork::new(
        args.value_of("kadcast_public_address").unwrap().to_string(),
        args.value_of("kadcast_listen_address")
            .map(|s| s.to_string()),
        args.values_of("kadcast_bootstrap")
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect(),
        args.is_present("kadcast_autobroadcast"),
    )
}
