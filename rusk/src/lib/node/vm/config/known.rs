// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Well-known VM configurations for different chain IDs.

use super::feature::{
    FEATURE_ABI_PUBLIC_SENDER, FEATURE_BLOB, FEATURE_DISABLE_WASM64,
    HQ_KECCAK256,
};
use super::{
    DEFAULT_BLOCK_GAS_LIMIT, DEFAULT_GAS_PER_BLOB, DEFAULT_GAS_PER_DEPLOY_BYTE,
    DEFAULT_MIN_DEPLOYMENT_GAS_PRICE, DEFAULT_MIN_DEPLOY_POINTS,
};

pub const MAINNET_ID: u8 = 1;
pub const TESTNET_ID: u8 = 2;
pub const DEVNET_ID: u8 = 3;

/// Contains well-known VM configurations for different chain IDs.
#[derive(Debug, Clone)]
pub struct WellKnownConfig {
    pub gas_per_blob: u64,
    pub gas_per_deploy_byte: u64,
    pub min_deploy_points: u64,
    pub min_deployment_gas_price: u64,
    pub block_gas_limit: u64,
    pub features: [(&'static str, u64); 4],
}

impl WellKnownConfig {
    /// Returns the well-known configuration for the given chain ID.
    ///
    /// If the chain ID is not recognized, returns the localnet configuration.
    pub fn from_chain_id(chain_id: u8) -> Self {
        match chain_id {
            MAINNET_ID => MAINNET_CONFIG,
            TESTNET_ID => TESTNET_CONFIG,
            DEVNET_ID => DEVNET_CONFIG,
            _ => LOCALNET_CONFIG,
        }
    }
}

/// Mainnet VM configuration.
const MAINNET_CONFIG: WellKnownConfig = WellKnownConfig {
    gas_per_blob: 0,
    gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
    min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
    min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
    block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
    features: [
        (FEATURE_ABI_PUBLIC_SENDER, 355_000),
        (HQ_KECCAK256, u64::MAX),
        (FEATURE_BLOB, u64::MAX),
        (FEATURE_DISABLE_WASM64, u64::MAX),
    ],
};

/// Testnet VM configuration.
const TESTNET_CONFIG: WellKnownConfig = WellKnownConfig {
    gas_per_blob: 0,
    gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
    min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
    min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
    block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
    features: [
        (FEATURE_ABI_PUBLIC_SENDER, 1),
        (HQ_KECCAK256, u64::MAX),
        (FEATURE_BLOB, 1),
        (FEATURE_DISABLE_WASM64, u64::MAX),
    ],
};

/// Devnet VM configuration.
const DEVNET_CONFIG: WellKnownConfig = WellKnownConfig {
    gas_per_blob: DEFAULT_GAS_PER_BLOB,
    gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
    min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
    min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
    block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
    features: [
        (FEATURE_ABI_PUBLIC_SENDER, 1),
        (HQ_KECCAK256, 1),
        (FEATURE_BLOB, 1),
        (FEATURE_DISABLE_WASM64, u64::MAX),
    ],
};

/// Localnet VM configuration.
const LOCALNET_CONFIG: WellKnownConfig = WellKnownConfig {
    gas_per_blob: DEFAULT_GAS_PER_BLOB,
    gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
    min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
    min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
    block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
    features: [
        (FEATURE_ABI_PUBLIC_SENDER, 1),
        (HQ_KECCAK256, 1),
        (FEATURE_BLOB, 1),
        (FEATURE_DISABLE_WASM64, 1),
    ],
};
