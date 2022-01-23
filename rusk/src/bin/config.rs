// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::ArgMatches;
use kadcast::config::Config as KadcastConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) ipc_method: Option<String>,
    pub(crate) socket: String,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) log_level: String,
    pub(crate) kadcast_test: bool,
    pub(crate) kadcast: KadcastConfig,
}

/// Default UDS path that Rusk GRPC-server will connect to.
pub const SOCKET_PATH: &str = "/tmp/rusk_listener";

/// Default port that Rusk GRPC-server will listen to.
pub(crate) const PORT: &str = "8585";
/// Default host_address that Rusk GRPC-server will listen to.
pub(crate) const HOST_ADDRESS: &str = "127.0.0.1";
/// Default log_level.
pub(crate) const LOG_LEVEL: &str = "info";

impl Default for Config {
    fn default() -> Self {
        Config {
            socket: SOCKET_PATH.to_string(),
            host: HOST_ADDRESS.to_string(),
            port: PORT.to_string(),
            log_level: LOG_LEVEL.to_string(),
            ipc_method: None,
            kadcast: KadcastConfig::default(),
            kadcast_test: false,
        }
    }
}

impl From<ArgMatches> for Config {
    fn from(matches: ArgMatches) -> Self {
        let mut rusk_config =
            matches
                .value_of("config")
                .map_or(Config::default(), |conf_path| {
                    let toml =
                        std::fs::read_to_string(conf_path.to_string()).unwrap();
                    toml::from_str(&toml).unwrap()
                });

        if let Some(log) = matches.value_of("log-level") {
            rusk_config.log_level = log.into();
        }
        if let Some(host) = matches.value_of("host") {
            rusk_config.host = host.into();
        }
        if let Some(port) = matches.value_of("port") {
            rusk_config.port = port.into();
        }
        if let Some(ipc_method) = matches.value_of("ipc_method") {
            rusk_config.ipc_method = Some(ipc_method.into());
        }
        if let Some(socket) = matches.value_of("socket") {
            rusk_config.socket = socket.into();
        }

        if let Some(public_address) = matches.value_of("kadcast_public_address")
        {
            rusk_config.kadcast.public_address = public_address.into();
        };
        if let Some(listen_address) = matches.value_of("kadcast_listen_address")
        {
            rusk_config.kadcast.listen_address = Some(listen_address.into());
        };
        if let Some(bootstrapping_nodes) =
            matches.values_of("kadcast_bootstrap")
        {
            rusk_config.kadcast.bootstrapping_nodes =
                bootstrapping_nodes.map(|s| s.into()).collect();
        };
        rusk_config.kadcast.auto_propagate =
            matches.is_present("kadcast_autobroadcast");
        rusk_config.kadcast_test = matches.is_present("kadcast_test");
        rusk_config
    }
}
