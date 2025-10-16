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
#[cfg(feature = "chain")]
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

// as the constants are only used in patterns, we need to declare them
// as dead code to avoid warnings
#[allow(dead_code)]
const MAINNET: u8 = 1;
#[allow(dead_code)]
const TESTNET: u8 = 2;
#[allow(dead_code)]
const DEVNET: u8 = 3;

// MAINNET constants
const MAINNET_GAS_PER_BLOB: u64 = 0;
const MAINNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
const MAINNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
const MAINNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
const MAINNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
const MAINNET_GENERATION_TIMEOUT: u64 = 3;

// TESTNET constants
const TESTNET_GAS_PER_BLOB: u64 = 0;
const TESTNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
const TESTNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
const TESTNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
const TESTNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
const TESTNET_GENERATION_TIMEOUT: u64 = 3;

// DEVNET constants
const DEVNET_GAS_PER_BLOB: u64 = 0;
const DEVNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
const DEVNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
const DEVNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
const DEVNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
const DEVNET_GENERATION_TIMEOUT: u64 = 3;

impl Config {
    #[cfg(feature = "chain")]
    fn set_values_for_the_known_chains(&mut self) {
        match self.kadcast.kadcast_id() {
            Some(MAINNET) => {
                self.vm.gas_per_blob =
                    self.vm.gas_per_blob.or(Some(MAINNET_GAS_PER_BLOB));
                self.vm.gas_per_deploy_byte = self
                    .vm
                    .gas_per_deploy_byte
                    .or(Some(MAINNET_GAS_PER_DEPLOY_BYTE));
                self.vm.min_deploy_points = self
                    .vm
                    .min_deploy_points
                    .or(Some(MAINNET_MIN_DEPLOY_POINTS));
                self.vm.min_deployment_gas_price = self
                    .vm
                    .min_deployment_gas_price
                    .or(Some(MAINNET_MIN_DEPLOYMENT_GAS_PRICE));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(MAINNET_BLOCK_GAS_LIMIT));
                self.vm.generation_timeout = self
                    .vm
                    .generation_timeout
                    .or(Some(Duration::from_secs(MAINNET_GENERATION_TIMEOUT)));
            }
            Some(TESTNET) => {
                self.vm.gas_per_blob =
                    self.vm.gas_per_blob.or(Some(TESTNET_GAS_PER_BLOB));
                self.vm.gas_per_deploy_byte = self
                    .vm
                    .gas_per_deploy_byte
                    .or(Some(TESTNET_GAS_PER_DEPLOY_BYTE));
                self.vm.min_deploy_points = self
                    .vm
                    .min_deploy_points
                    .or(Some(TESTNET_MIN_DEPLOY_POINTS));
                self.vm.min_deployment_gas_price = self
                    .vm
                    .min_deployment_gas_price
                    .or(Some(TESTNET_MIN_DEPLOYMENT_GAS_PRICE));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(TESTNET_BLOCK_GAS_LIMIT));
                self.vm.generation_timeout = self
                    .vm
                    .generation_timeout
                    .or(Some(Duration::from_secs(TESTNET_GENERATION_TIMEOUT)));
            }
            Some(DEVNET) => {
                self.vm.gas_per_blob =
                    self.vm.gas_per_blob.or(Some(DEVNET_GAS_PER_BLOB));
                self.vm.gas_per_deploy_byte = self
                    .vm
                    .gas_per_deploy_byte
                    .or(Some(DEVNET_GAS_PER_DEPLOY_BYTE));
                self.vm.min_deploy_points = self
                    .vm
                    .min_deploy_points
                    .or(Some(DEVNET_MIN_DEPLOY_POINTS));
                self.vm.min_deployment_gas_price = self
                    .vm
                    .min_deployment_gas_price
                    .or(Some(DEVNET_MIN_DEPLOYMENT_GAS_PRICE));
                self.vm.block_gas_limit =
                    self.vm.block_gas_limit.or(Some(DEVNET_BLOCK_GAS_LIMIT));
                self.vm.generation_timeout = self
                    .vm
                    .generation_timeout
                    .or(Some(Duration::from_secs(DEVNET_GENERATION_TIMEOUT)));
            }
            _ => {}
        }
    }
    #[cfg(not(feature = "chain"))]
    fn set_values_for_the_known_chains(&mut self) {}
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
