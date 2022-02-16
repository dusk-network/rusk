// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct GrpcConfig {
    pub(crate) ipc_method: Option<String>,
    pub(crate) socket: String,
    pub(crate) host: String,
    pub(crate) port: String,
}

/// Default UDS path that Rusk GRPC-server will connect to.
pub const SOCKET_PATH: &str = "/tmp/rusk_listener";

/// Default port that Rusk GRPC-server will listen to.
pub(crate) const PORT: &str = "8585";
/// Default host_address that Rusk GRPC-server will listen to.
pub(crate) const HOST_ADDRESS: &str = "127.0.0.1";

impl Default for GrpcConfig {
    fn default() -> Self {
        GrpcConfig {
            socket: SOCKET_PATH.to_string(),
            host: HOST_ADDRESS.to_string(),
            port: PORT.to_string(),
            ipc_method: None,
        }
    }
}

impl GrpcConfig {
    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        if let Some(host) = matches.value_of("host") {
            self.host = host.into();
        }
        if let Some(port) = matches.value_of("port") {
            self.port = port.into();
        }
        if let Some(ipc_method) = matches.value_of("ipc_method") {
            self.ipc_method = Some(ipc_method.into());
        }
        if let Some(socket) = matches.value_of("socket") {
            self.socket = socket.into();
        }
    }
    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        command
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
                    .help(
                        "Inter-Process communication protocol you want to use ",
                    )
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
    }
}
