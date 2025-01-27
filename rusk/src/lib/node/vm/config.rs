// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_vm::ExecutionConfig;

const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;
const DEFAULT_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2000;
const DEFAULT_MIN_DEPLOY_POINTS: u64 = 5_000_000;

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
            min_deploy_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
            min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
        }
    }
}

impl Config {
    /// Create a new `Config` with the given parameters.
    pub fn to_execution_config(&self) -> ExecutionConfig {
        ExecutionConfig {
            gas_per_deploy_byte: self.gas_per_deploy_byte,
            min_deploy_points: self.min_deploy_points,
            min_deploy_gas_price: self.min_deploy_gas_price,
        }
    }
}
