// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod kadcast;

use std::path::PathBuf;
use std::str::FromStr;

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

use self::kadcast::KadcastConfig;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct Config {
    log_level: Option<String>,
    log_type: Option<String>,
    pub(crate) network: KadcastConfig,
    db_path: Option<PathBuf>,
    consensus_keys_path: Option<PathBuf>,
}

/// Default log_level.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log_type.
const DEFAULT_LOG_TYPE: &str = "coloured";

impl From<&ArgMatches> for Config {
    fn from(matches: &ArgMatches) -> Self {
        let mut config =
            matches
                .value_of("config")
                .map_or(Config::default(), |conf_path| {
                    let toml = std::fs::read_to_string(conf_path).unwrap();
                    toml::from_str(&toml).unwrap()
                });

        // Overwrite config log-level
        if let Some(log_level) = matches.value_of("log-level") {
            config.log_level = Some(log_level.into());
        }

        // Overwrite config log-type
        if let Some(log_type) = matches.value_of("log-type") {
            config.log_type = Some(log_type.into());
        }

        // Overwrite config consensus-keys-path
        if let Some(consensus_keys_path) =
            matches.value_of("consensus-keys-path")
        {
            config.consensus_keys_path =
                Some(PathBuf::from(consensus_keys_path));
        }

        // Overwrite config db-path
        if let Some(db_path) = matches.value_of("db-path") {
            config.db_path = Some(PathBuf::from(db_path));
        }

        config.network.merge(matches);
        config
    }
}

impl Config {
    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        let command = KadcastConfig::inject_args(command);
        command
            .arg(
                Arg::new("log-level")
                    .long("log-level")
                    .value_name("LOG")
                    .possible_values([
                        "error", "warn", "info", "debug", "trace",
                    ])
                    .help("Output log level")
                    .takes_value(true),
            )
            .arg(
                Arg::new("log-type")
                    .long("log-type")
                    .value_name("LOG_TYPE")
                    .possible_values(["coloured", "plain", "json"])
                    .help("Change the log format accordingly")
                    .takes_value(true),
            )
            .arg(
                Arg::new("consensus-keys-path")
                    .long("consensus-keys-path")
                    .value_name("CONSENSUS_KEYS_PATH")
                    .help("path to encrypted BLS keys")
                    .takes_value(true),
            )
            .arg(
                Arg::new("db-path")
                    .long("db-path")
                    .value_name("DB_PATH")
                    .help("path to blockchain database")
                    .takes_value(true),
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
            panic!("Invalid log-level specified '{log_level}' - {e}")
        })
    }

    pub(crate) fn db_path(&self) -> PathBuf {
        self.db_path.clone().unwrap_or_else(|| {
            let mut path = dirs::home_dir().expect("OS not supported");
            path.push(".dusk");
            path.push(env!("CARGO_BIN_NAME"));
            path
        })
    }

    pub(crate) fn consensus_keys_path(&self) -> String {
        self.consensus_keys_path
            .clone()
            .unwrap_or_else(|| {
                let mut path = dirs::home_dir().expect("OS not supported");
                path.push(".dusk");
                path.push("consensus.keys");
                path
            })
            .as_path()
            .display()
            .to_string()
    }
}
