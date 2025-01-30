// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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
    /// Enable the public sender metadata in the transaction.
    ///
    /// This field may be deprecated after the feature rollout.
    pub with_public_sender: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Config {
    /// Create a config with all values to default
    pub const DEFAULT: Config = Config {
        gas_per_deploy_byte: 0,
        min_deploy_points: 0,
        min_deploy_gas_price: 0,
        with_public_sender: false,
    };
}
