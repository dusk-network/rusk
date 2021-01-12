// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(not(target_os = "windows"))]
mod unix;
mod version;

use canonical_host::{MemStore, Remote, Wasm};
use clap::{App, Arg};
use dusk_bls12_381_sign::{PublicKey, SecretKey, APK};
use futures::stream::TryStreamExt;
use reward_contract::{Contract, PublicKeys};
use rusk::services::blindbid::BlindBidServiceServer;
use rusk::services::echoer::EchoerServer;
use rusk::services::pki::KeysServer;
use rusk::Rusk;
use rusk::{RuskExternalError, RuskExternals};
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

const BYTECODE: &'static [u8] = include_bytes!(
    "../../contracts/reward/target/wasm32-unknown-unknown/release/reward_contract.wasm"
);

fn main() {
    // Init Env & Contract
    let store = MemStore::new();
    let wasm_contract = Wasm::new(Contract::new(), BYTECODE);
    let mut remote = Remote::new(wasm_contract, &store).unwrap();

    let value = 100u64;
    // Create 128 public keys
    let mut keys = [APK::default(); 10];
    for i in 0..keys.len() {
        let sk = SecretKey::new(&mut rand_core::OsRng);
        let pk = PublicKey::from(&sk);
        let apk = APK::from(&pk);
        keys[i] = apk;
    }

    let pks = PublicKeys::from(keys.clone());

    // Call distribute
    let mut cast = remote
        .cast_mut::<Wasm<Contract<MemStore>, MemStore>>()
        .unwrap();
    let res = cast
        .transact(
            &Contract::<MemStore>::distribute(value, pks),
            store.clone(),
            RuskExternals::default(),
        )
        .expect("Failed to call the distribute fn");
    // If call succeeds, this should not fail.
    cast.commit().expect("Commit couldn't be done");
}
/*
#[tokio::main]
async fn main() {
    let crate_info = get_version_info!();
    let matches = App::new(&crate_info.crate_name)
        .version(show_version(crate_info).as_str())
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
        )
        .get_matches();

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

    // Match the desired IPC method. Or set the default one depending on the OS
    // used. Then startup rusk with the final values.
    let res = match matches.value_of("ipc_method") {
        Some(method) => match (cfg!(windows), method) {
            (_, "tcp_ip") => {
                startup_with_tcp_ip(
                    matches.value_of("host").unwrap_or(HOST_ADDRESS),
                    matches.value_of("port").unwrap_or(PORT),
                )
                .await
            }
            (true, "uds") => {
                panic!("Windows does not support Unix Domain Sockets");
            }
            (false, "uds") => {
                startup_with_uds(
                    matches.value_of("socket").unwrap_or(SOCKET_PATH),
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
                )
                .await
            } else {
                startup_with_uds(
                    matches.value_of("socket").unwrap_or(SOCKET_PATH),
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
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(Path::new(path).parent().unwrap()).await?;

    let mut uds = UnixListener::bind(path)?;

    let rusk = Rusk::default();

    let echoer = EchoerServer::new(rusk);
    let blindbid = BlindBidServiceServer::new(rusk);
    let keys = KeysServer::new(rusk);
    Server::builder()
        .add_service(echoer)
        .add_service(blindbid)
        .add_service(keys)
        .serve_with_incoming(uds.incoming().map_ok(unix::UnixStream))
        .await?;

    Ok(())
}

async fn startup_with_tcp_ip(
    host: &str,
    port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut full_address = host.to_string();
    full_address.push(':');
    full_address.push_str(&port.to_string());
    let addr: std::net::SocketAddr = full_address.parse()?;
    let rusk = Rusk::default();

    let echoer = EchoerServer::new(rusk);
    let blindbid = BlindBidServiceServer::new(rusk);
    let keys = KeysServer::new(rusk);

    // Build the Server with the `Echo` service attached to it.
    Ok(Server::builder()
        .add_service(echoer)
        .add_service(blindbid)
        .add_service(keys)
        .serve(addr)
        .await?)
}
*/
