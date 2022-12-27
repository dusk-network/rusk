// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod grpc;
pub mod kadcast;

use std::str::FromStr;

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

use self::{grpc::GrpcConfig, kadcast::KadcastConfig};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct Config {
    log_level: Option<String>,
    log_type: Option<String>,
    pub(crate) kadcast_test: bool,
    pub(crate) grpc: GrpcConfig,
    pub(crate) kadcast: KadcastConfig,
}

/// Default log_level.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log_type.  
const DEFAULT_LOG_TYPE: &str = "coloured";

impl From<&ArgMatches> for Config {
    fn from(matches: &ArgMatches) -> Self {
        let mut rusk_config = matches.get_one::<String>("config").map_or(
            Config::default(),
            |conf_path| {
                let toml = std::fs::read_to_string(conf_path).unwrap();
                toml::from_str(&toml).unwrap()
            },
        );

        rusk_config.kadcast_test = matches.contains_id("kadcast_test");

        // Overwrite config log-level
        if let Some(log_level) = matches.get_one::<String>("log-level").cloned()
        {
            rusk_config.log_level = Some(log_level);
        }
        // Overwrite config log-type
        if let Some(log_type) = matches.get_one::<String>("log-type").cloned() {
            rusk_config.log_type = Some(log_type);
        }

        rusk_config.grpc.merge(matches);
        rusk_config.kadcast.merge(matches);
        rusk_config
    }
}

impl Config {
    pub fn inject_args(command: Command) -> Command {
        let command = KadcastConfig::inject_args(command);
        let command = GrpcConfig::inject_args(command);
        command.arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LOG")
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .help("Output log level")
                .num_args(1),
        )
        .arg(
            Arg::new("log-type")
                .long("log-type")
                .value_name("LOG_TYPE")
                .value_parser(["coloured", "plain", "json"])
                .help("Change the log format accordingly")
                .num_args(1),
        )
        .arg(
            Arg::new("kadcast_test")
                .long("kadcast_test")
                .env("KADCAST_TEST")
                .help("If used then the received messages is a blake2b 256hash")
                .num_args(0)
                .required(false),
        )
    }

    pub(crate) fn log_type(&self) -> String {
        match &self.log_type {
            None => DEFAULT_LOG_TYPE.into(),
            Some(log_type) => log_type.into(),
        }
    }

    pub(crate) fn log_level(&self) -> tracing::Level {
        let log_level = match &self.log_level {
            None => DEFAULT_LOG_LEVEL,
            Some(log_level) => log_level,
        };
        tracing::Level::from_str(log_level).unwrap_or_else(|e| {
            panic!("Invalid log-level specified '{}' - {}", log_level, e)
        })
    }
}
