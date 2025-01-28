// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use dusk_vm::ExecutionConfig;

const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;
const DEFAULT_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2000;
const DEFAULT_MIN_DEPLOY_POINTS: u64 = 5_000_000;
const DEFAULT_BLOCK_GAS_LIMIT: u64 = 3 * 1_000_000_000;

/// Configuration for the execution of a transaction.
#[derive(Debug, Clone)]
pub struct Config {
    /// The amount of gas points charged for each byte in a contract-deployment
    /// bytecode.
    pub gas_per_deploy_byte: u64,
    /// The minimum gas points charged for a contract deployment.
    pub min_deploy_points: u64,
    /// The minimum gas price set for a contract deployment
    pub min_deploy_gas_price: u64,
    /// The maximum amount of gas points that can be used in a block.
    pub block_gas_limit: u64,
    /// The timeout for a candidate block generation.
    pub generation_timeout: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub const fn new() -> Self {
        Self {
            gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
            min_deploy_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
            min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
            block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
            generation_timeout: None,
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
        self.min_deploy_gas_price = min_deploy_gas_price;
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
    pub fn to_execution_config(&self) -> ExecutionConfig {
        ExecutionConfig {
            gas_per_deploy_byte: self.gas_per_deploy_byte,
            min_deploy_points: self.min_deploy_points,
            min_deploy_gas_price: self.min_deploy_gas_price,
        }
    }
}
