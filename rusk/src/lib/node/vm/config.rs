// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod known;
pub mod opt;

use std::collections::{hash_map::Iter, HashMap};
use std::time::Duration;

use dusk_vm::ExecutionConfig;

const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;

// TODO: This is a temporary value. Change this value to the tuned one as soon
// as it's rolled out.
const DEFAULT_GAS_PER_BLOB: u64 = 1_000_000;

const DEFAULT_MIN_DEPLOY_POINTS: u64 = 5_000_000;
const DEFAULT_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
const DEFAULT_BLOCK_GAS_LIMIT: u64 = 3 * 1_000_000_000;

/// Configuration for the execution of a transaction.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Config {
    /// The amount of gas points charged for each blob in a transaction
    pub gas_per_blob: u64,

    /// The amount of gas points charged for each byte in a contract-deployment
    /// bytecode.
    pub gas_per_deploy_byte: u64,

    /// The minimum gas points charged for a contract deployment.
    pub min_deploy_points: u64,

    /// The minimum gas price set for a contract deployment
    pub min_deployment_gas_price: u64,

    /// The maximum amount of gas points that can be used in a block.
    pub block_gas_limit: u64,

    /// The timeout for a candidate block generation.
    #[serde(with = "humantime_serde")]
    pub generation_timeout: Option<Duration>,

    /// Set of features to activate
    features: HashMap<String, u64>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) mod feature {
    pub const FEATURE_ABI_PUBLIC_SENDER: &str = "ABI_PUBLIC_SENDER";
    pub const FEATURE_BLOB: &str = "BLOB";
    pub const FEATURE_DISABLE_WASM64: &str = "DISABLE_WASM64";
    pub const HQ_KECCAK256: &str = "HQ_KECCAK256";
}

impl Config {
    pub fn new() -> Self {
        Self {
            gas_per_blob: DEFAULT_GAS_PER_BLOB,
            gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
            min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
            min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
            block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
            generation_timeout: None,
            features: HashMap::new(),
        }
    }

    /// Set the maximum amount of gas points that can be used in a block.
    pub const fn with_block_gas_limit(mut self, block_gas_limit: u64) -> Self {
        self.block_gas_limit = block_gas_limit;
        self
    }

    /// Set the amount of gas points charged for each byte in a
    /// contract-deployment
    pub const fn with_gas_per_deploy_byte(
        mut self,
        gas_per_deploy_byte: u64,
    ) -> Self {
        self.gas_per_deploy_byte = gas_per_deploy_byte;
        self
    }

    /// Set the minimum amount of gas points charged for a contract deployment.
    pub const fn with_min_deploy_points(
        mut self,
        min_deploy_points: u64,
    ) -> Self {
        self.min_deploy_points = min_deploy_points;
        self
    }

    /// Set the minimum gas price set for a contract deployment.
    pub const fn with_min_deploy_gas_price(
        mut self,
        min_deploy_gas_price: u64,
    ) -> Self {
        self.min_deployment_gas_price = min_deploy_gas_price;
        self
    }

    /// Set the timeout for a candidate block generation.
    pub const fn with_generation_timeout(
        mut self,
        generation_timeout: Option<Duration>,
    ) -> Self {
        self.generation_timeout = generation_timeout;
        self
    }

    /// Create a new `Config` with the given parameters.
    pub fn to_execution_config(&self, block_height: u64) -> ExecutionConfig {
        let with_public_sender: bool = self
            .feature(feature::FEATURE_ABI_PUBLIC_SENDER)
            .map(|activation| block_height >= activation)
            .unwrap_or_default();
        let with_blob = self
            .feature(feature::FEATURE_BLOB)
            .map(|activation| block_height >= activation)
            .unwrap_or_default();
        let disable_wasm64 = self
            .feature(feature::FEATURE_DISABLE_WASM64)
            .map(|activation| block_height >= activation)
            .unwrap_or_default();
        ExecutionConfig {
            gas_per_blob: self.gas_per_blob,
            gas_per_deploy_byte: self.gas_per_deploy_byte,
            min_deploy_points: self.min_deploy_points,
            min_deploy_gas_price: self.min_deployment_gas_price,
            with_public_sender,
            with_blob,
            disable_wasm64,
        }
    }

    pub fn features(&self) -> Iter<String, u64> {
        self.features.iter()
    }

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
        let feature = self
            .features
            .keys()
            .find(|k| k.eq_ignore_ascii_case(&feature))
            .cloned()
            .unwrap_or(feature);
        self.features.insert(feature, activation);
    }
}
