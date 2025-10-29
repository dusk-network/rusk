// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{known::WellKnownConfig, Config};

/// Configuration for the execution of a transaction.
///
/// All fields are optional. When converting to [Config] with
/// [Config::try_from], all fields must be set, otherwise an error is returned.
///
/// This struct allows to load partial configuration from external sources,
/// such as configuration files or network well-known configurations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OptionalConfig {
    /// The amount of gas points charged for each blob in a transaction
    pub gas_per_blob: Option<u64>,

    /// The amount of gas points charged for each byte in a contract-deployment
    /// bytecode.
    pub gas_per_deploy_byte: Option<u64>,

    /// The minimum gas points charged for a contract deployment.
    pub min_deploy_points: Option<u64>,

    /// The minimum gas price set for a contract deployment
    pub min_deployment_gas_price: Option<u64>,

    /// The maximum amount of gas points that can be used in a block.
    pub block_gas_limit: Option<u64>,

    /// The timeout for a candidate block generation.
    #[serde(default, with = "humantime_serde")]
    pub generation_timeout: Option<Duration>,

    /// Set of features to activate
    #[serde(default)]
    features: HashMap<String, u64>,
}

impl OptionalConfig {
    pub fn feature(&self, feature: &str) -> Option<u64> {
        self.features
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(feature))
            .map(|(_, &v)| v)
    }

    pub fn with_feature<S: Into<String>>(
        &mut self,
        feature: S,
        activation: u64,
    ) {
        let feature: String = feature.into();
        // Check for case insensitive key
        if self.feature(&feature).is_some() {
            self.features.remove(&feature);
        }
        self.features.insert(feature, activation);
    }

    /// Injects the well-known configuration into this optional configuration.
    ///
    /// If a field is already set, a warning is logged and the existing value is
    /// kept.
    /// If a feature is already set, a warning is logged and the existing value
    /// is kept.
    /// If a feature is not recognized in the well-known configuration, a
    /// warning is logged.
    pub fn inject_network_conf(&mut self, config: WellKnownConfig) {
        Self::set_or_warn(
            "gas_per_blob",
            &mut self.gas_per_blob,
            config.gas_per_blob,
        );
        Self::set_or_warn(
            "gas_per_deploy_byte",
            &mut self.gas_per_deploy_byte,
            config.gas_per_deploy_byte,
        );
        Self::set_or_warn(
            "min_deploy_points",
            &mut self.min_deploy_points,
            config.min_deploy_points,
        );
        Self::set_or_warn(
            "min_deployment_gas_price",
            &mut self.min_deployment_gas_price,
            config.min_deployment_gas_price,
        );

        Self::set_or_warn(
            "block_gas_limit",
            &mut self.block_gas_limit,
            config.block_gas_limit,
        );

        for (feature, activation) in &config.features {
            if let Some(v) = self.feature(feature) {
                if v != *activation {
                    warn!("[vm].feature {feature} is set to {v} (overriding the well-known network config value of {activation})");
                }
            } else {
                self.with_feature(*feature, *activation);
            }
        }

        for feature in self.features.keys() {
            if !config.features.iter().any(|(known_feature, _)| {
                known_feature.eq_ignore_ascii_case(feature)
            }) {
                warn!("[vm].feature {feature} is not recognized in the well-known network config");
            }
        }
    }

    fn set_or_warn<T>(field_name: &str, field: &mut Option<T>, config_value: T)
    where
        T: Copy + PartialEq + std::fmt::Display,
    {
        if let Some(current) = field {
            if *current != config_value {
                warn!("[vm].{field_name} is set to {current} (overriding the well-known network config value of {config_value})");
            }
        } else {
            let _ = field.insert(config_value);
        }
    }
}

impl TryFrom<OptionalConfig> for Config {
    type Error = anyhow::Error;

    fn try_from(value: OptionalConfig) -> Result<Self, Self::Error> {
        Ok(Config {
            gas_per_blob: value
                .gas_per_blob
                .ok_or(anyhow!("Missing gas_per_blob"))?,
            gas_per_deploy_byte: value
                .gas_per_deploy_byte
                .ok_or(anyhow!("Missing gas_per_deploy_byte"))?,
            min_deploy_points: value
                .min_deploy_points
                .ok_or(anyhow!("Missing min_deploy_points"))?,
            min_deployment_gas_price: value
                .min_deployment_gas_price
                .ok_or(anyhow!("Missing min_deployment_gas_price"))?,
            block_gas_limit: value
                .block_gas_limit
                .ok_or(anyhow!("Missing block_gas_limit"))?,
            generation_timeout: value.generation_timeout,
            features: value.features,
        })
    }
}
