// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod grpc;
pub mod kadcast;
pub mod wallet;

use clap::{App, Arg, ArgMatches};
use serde::{Deserialize, Serialize};

use self::{grpc::GrpcConfig, kadcast::KadcastConfig, wallet::WalletConfig};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) log_level: String,
    pub(crate) kadcast_test: bool,
    pub(crate) grpc: GrpcConfig,
    pub(crate) wallet: WalletConfig,
    pub(crate) kadcast: KadcastConfig,
}

/// Default log_level.
pub(crate) const LOG_LEVEL: &str = "info";

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: LOG_LEVEL.to_string(),
            kadcast_test: false,
            grpc: GrpcConfig::default(),
            kadcast: KadcastConfig::default(),
            wallet: WalletConfig::default(),
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

        rusk_config.kadcast_test = matches.is_present("kadcast_test");
        if let Some(log) = matches.value_of("log-level") {
            rusk_config.log_level = log.into();
        }

        rusk_config.grpc.merge(&matches);
        rusk_config.kadcast.merge(&matches);
        rusk_config.wallet.merge(&matches);
        rusk_config
    }
}

impl Config {
    pub fn inject_args(app: App<'_>) -> App<'_> {
        let app = KadcastConfig::inject_args(app);
        let app = GrpcConfig::inject_args(app);
        let app = WalletConfig::inject_args(app);
        app.arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LOG")
                .possible_values(&["error", "warn", "info", "debug", "trace"])
                .help("Output log level")
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
