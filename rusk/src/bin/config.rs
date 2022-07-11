// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod grpc;
pub mod kadcast;

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

use self::{grpc::GrpcConfig, kadcast::KadcastConfig};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) log_level: String,
    pub(crate) log_type: String,
    pub(crate) kadcast_test: bool,
    pub(crate) grpc: GrpcConfig,
    pub(crate) kadcast: KadcastConfig,
}

/// Default log_level.
pub(crate) const LOG_LEVEL: &str = "info";

/// Default log_type.
pub(crate) const LOG_TYPE: &str = "coloured";

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: LOG_LEVEL.to_string(),
            log_type: LOG_TYPE.to_string(),
            kadcast_test: false,
            grpc: GrpcConfig::default(),
            kadcast: KadcastConfig::default(),
        }
    }
}

impl From<ArgMatches> for Config {
    fn from(matches: ArgMatches) -> Self {
        let mut rusk_config =
            matches
                .value_of("config")
                .map_or(Config::default(), |conf_path| {
                    let toml = std::fs::read_to_string(conf_path).unwrap();
                    toml::from_str(&toml).unwrap()
                });

        rusk_config.kadcast_test = matches.is_present("kadcast_test");
        if let Some(log) = matches.value_of("log-level") {
            rusk_config.log_level = log.into();
        }
        if let Some(log_type) = matches.value_of("log-type") {
            rusk_config.log_type = log_type.into();
        }

        rusk_config.grpc.merge(&matches);
        rusk_config.kadcast.merge(&matches);
        rusk_config
    }
}

impl Config {
    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        let command = KadcastConfig::inject_args(command);
        let command = GrpcConfig::inject_args(command);
        command.arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LOG")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .help("Output log level")
                .takes_value(true),
        )
        .arg(
            Arg::new("log-type")
                .long("log-type")
                .value_name("LOG_TYPE")
                .possible_values(&["coloured", "plan", "json"])
                .help("Change the log format accordingly")
                .default_value("coloured")
                .takes_value(true),
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
}
