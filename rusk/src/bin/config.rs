// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod chain;
pub mod http;
pub mod kadcast;

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{value_parser, Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

use self::chain::ChainConfig;
use self::http::HttpConfig;
use self::kadcast::KadcastConfig;

type DataBrokerConfig = node::databroker::conf::Params;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct Config {
    log_level: Option<String>,
    log_type: Option<String>,
    log_filter: Option<String>,

    pub(crate) databroker: DataBrokerConfig,

    pub(crate) kadcast: KadcastConfig,
    pub(crate) chain: ChainConfig,
    #[serde(default = "HttpConfig::default")]
    pub(crate) http: HttpConfig,
}

/// Default log_level.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log_type.
const DEFAULT_LOG_TYPE: &str = "coloured";

impl From<&ArgMatches> for Config {
    fn from(matches: &ArgMatches) -> Self {
        let mut rusk_config =
            matches
                .value_of("config")
                .map_or(Config::default(), |conf_path| {
                    let toml = std::fs::read_to_string(conf_path).unwrap();
                    toml::from_str(&toml).unwrap()
                });

        // Overwrite config log-level
        if let Some(log_level) = matches.value_of("log-level") {
            rusk_config.log_level = Some(log_level.into());
        }

        // Overwrite config log-type
        if let Some(log_type) = matches.value_of("log-type") {
            rusk_config.log_type = Some(log_type.into());
        }

        // Overwrite config log-filter
        if let Some(log_filter) = matches.value_of("log-filter") {
            rusk_config.log_filter = Some(log_filter.into());
        }

        // Set profile path if specified
        if let Some(profile) = matches.value_of("profile-path") {
            // Since the profile path is resolved by the rusk_profile library,
            // there is the need to set the env variable
            env::set_var("RUSK_PROFILE_PATH", profile);
        }

        rusk_config.kadcast.merge(matches);
        rusk_config.chain.merge(matches);
        rusk_config.http.merge(matches);
        rusk_config.databroker.merge(matches);
        rusk_config
    }
}

impl Config {
    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        let command = KadcastConfig::inject_args(command);
        let command = ChainConfig::inject_args(command);
        let command = HttpConfig::inject_args(command);
        let command = DataBrokerConfig::inject_args(command);
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
                Arg::new("log-filter")
                    .long("log-filter")
                    .value_name("LOG_FILTER")
                    .help("Add log filter(s)")
                    .takes_value(true),
            )
            .arg(
                Arg::new("profile-path")
                    .long("profile")
                    .help("Sets the profile path")
                    .takes_value(true)
                    .value_parser(value_parser!(PathBuf))
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
            panic!("Invalid log-level specified '{log_level}' - {e}")
        })
    }

    pub(crate) fn log_filter(&self) -> String {
        match &self.log_filter {
            None => "".to_owned(),
            Some(log_filter) => log_filter.into(),
        }
    }

    pub(crate) fn databroker(&self) -> &node::databroker::conf::Params {
        &self.databroker
    }
}
