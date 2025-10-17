// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "chain")]
pub mod blob;
#[cfg(feature = "chain")]
pub mod chain;
#[cfg(feature = "chain")]
pub mod databroker;
#[cfg(feature = "chain")]
pub mod kadcast;
#[cfg(feature = "chain")]
pub mod mempool;
#[cfg(feature = "chain")]
pub mod telemetry;

pub mod http;

use std::env;
use std::str::FromStr;
use std::time::Duration;

#[cfg(feature = "chain")]
use self::{
    blob::BlobConfig, chain::ChainConfig, databroker::DataBrokerConfig,
    kadcast::KadcastConfig, mempool::MempoolConfig, telemetry::TelemetryConfig,
};

#[cfg(feature = "chain")]
use rusk::node::RuskVmConfig;

use serde::{Deserialize, Serialize};

use crate::args::Args;

use self::http::HttpConfig;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct Config {
    log_level: Option<String>,
    log_type: Option<String>,
    log_filter: Option<String>,

    #[cfg(feature = "chain")]
    #[serde(default = "DataBrokerConfig::default")]
    pub(crate) databroker: DataBrokerConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "KadcastConfig::default")]
    pub(crate) kadcast: KadcastConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "ChainConfig::default")]
    pub(crate) chain: ChainConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "RuskVmConfig::default")]
    pub(crate) vm: RuskVmConfig,

    #[serde(default = "HttpConfig::default")]
    pub(crate) http: HttpConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "TelemetryConfig::default")]
    pub(crate) telemetry: TelemetryConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "MempoolConfig::default")]
    pub(crate) mempool: MempoolConfig,

    #[cfg(feature = "chain")]
    #[serde(default = "BlobConfig::default")]
    pub(crate) blob: BlobConfig,
}

/// Default log_level.
const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log_type.
const DEFAULT_LOG_TYPE: &str = "coloured";

impl From<&Args> for Config {
    fn from(args: &Args) -> Self {
        let mut rusk_config =
            args.config.as_ref().map_or(Config::default(), |conf_path| {
                let toml = std::fs::read_to_string(conf_path).unwrap();
                let mut cfg: Config = toml::from_str(&toml).unwrap();
                #[cfg(feature = "chain")]
                cfg.set_values_for_the_known_chains();
                cfg
            });

        // Overwrite config log-level
        if let Some(log_level) = args.log_level {
            rusk_config.log_level = Some(log_level.to_string());
        }

        // Overwrite config log-type
        if let Some(log_type) = &args.log_type {
            rusk_config.log_type = Some(log_type.into());
        }

        // Overwrite config log-filter
        if let Some(log_filter) = &args.log_filter {
            rusk_config.log_filter = Some(log_filter.into());
        }

        // Set profile path if specified
        if let Some(profile) = &args.profile {
            // Since the profile path is resolved by the rusk_profile library,
            // there is the need to set the env variable
            env::set_var("RUSK_PROFILE_PATH", profile);
        }

        rusk_config.http.merge(args);

        #[cfg(feature = "chain")]
        {
            rusk_config.kadcast.merge(args);
            rusk_config.chain.merge(args);
            rusk_config.databroker.merge(args);
            rusk_config.telemetry.merge(args);
        }

        rusk_config
    }
}

// todo: move it to the right place
// as the constants are only used in patterns, we need to declare them
// as dead code to avoid warnings, should the constants become public
// this problem will disappear
#[allow(dead_code)]
const MAINNET: u8 = 1;
#[allow(dead_code)]
const TESTNET: u8 = 2;
#[allow(dead_code)]
const DEVNET: u8 = 3;

impl Config {
    #[cfg(feature = "chain")]
    fn set_values_for_the_known_chains(&mut self) {
        // let kadcast_config: kadcast_config::Config = self.kadcast.clone().into();
        let kadcast_id: u8 = 1;
        // todo: how to extract kadcast id from KadcastConfig?
        match kadcast_id {
            MAINNET => {
                self.vm.gas_per_blob = self.vm.gas_per_blob.or(Some(0));
                self.vm.gas_per_deploy_byte =
                    self.vm.gas_per_deploy_byte.or(Some(100));
                self.vm.min_deploy_points =
                    self.vm.min_deploy_points.or(Some(5_000_000));
                self.vm.min_deployment_gas_price =
                    self.vm.min_deployment_gas_price.or(Some(2_000));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(3_000_000_000));
                self.vm.generation_timeout =
                    self.vm.generation_timeout.or(Some(Duration::from_secs(3)));
            }
            TESTNET => {
                self.vm.gas_per_blob = self.vm.gas_per_blob.or(Some(0));
                self.vm.gas_per_deploy_byte =
                    self.vm.gas_per_deploy_byte.or(Some(100));
                self.vm.min_deploy_points =
                    self.vm.min_deploy_points.or(Some(5_000_000));
                self.vm.min_deployment_gas_price =
                    self.vm.min_deployment_gas_price.or(Some(2_000));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(3_000_000_000));
                self.vm.generation_timeout =
                    self.vm.generation_timeout.or(Some(Duration::from_secs(3)));
            }
            DEVNET => {
                self.vm.gas_per_blob = self.vm.gas_per_blob.or(Some(0));
                self.vm.gas_per_deploy_byte =
                    self.vm.gas_per_deploy_byte.or(Some(100));
                self.vm.min_deploy_points =
                    self.vm.min_deploy_points.or(Some(5_000_000));
                self.vm.min_deployment_gas_price =
                    self.vm.min_deployment_gas_price.or(Some(2_000));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(3_000_000_000));
                self.vm.generation_timeout =
                    self.vm.generation_timeout.or(Some(Duration::from_secs(3)));
            }
            _ => {}
        }
    }
}

impl Config {
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
        self.log_filter.clone().unwrap_or_default()
    }
}
