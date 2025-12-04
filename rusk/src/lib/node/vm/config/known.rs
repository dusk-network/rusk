// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Well-known VM configurations for different chain IDs.

use std::sync::LazyLock;

use dusk_vm::FeatureActivation;

use crate::node::{FEATURE_DISABLE_3RD_PARTY, FEATURE_DISABLE_WASM32};

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

const GENESIS: FeatureActivation = FeatureActivation::Height(1);
const NEVER: FeatureActivation = FeatureActivation::Height(u64::MAX);

/// Contains well-known VM configurations for different chain IDs.
#[derive(Debug, Clone)]
pub struct WellKnownConfig {
    pub gas_per_blob: u64,
    pub gas_per_deploy_byte: u64,
    pub min_deploy_points: u64,
    pub min_deployment_gas_price: u64,
    pub block_gas_limit: u64,
    pub features: [(&'static str, FeatureActivation); 6],
}

impl WellKnownConfig {
    /// Returns the well-known configuration for the given chain ID.
    ///
    /// If the chain ID is not recognized, returns the localnet configuration.
    pub fn from_chain_id(chain_id: u8) -> Self {
        match chain_id {
            MAINNET_ID => MAINNET_CONFIG.clone(),
            TESTNET_ID => TESTNET_CONFIG,
            DEVNET_ID => DEVNET_CONFIG,
            _ => LOCALNET_CONFIG,
        }
    }
}

/// Estimated mainnet block height for 10th December 2025, 09:00 UTC.
const MAINNET_AT_10_12_2025_AT_09_00_UTC: u64 = 2_873_420;

const MAINNET_SENDER_ACTIVATION_HEIGHT: FeatureActivation =
    FeatureActivation::Height(355_000);

const MAINNET_DISABLED_3D_PARTY_START: u64 = 2_710_376;
const MAINNET_DISABLED_3D_PARTY_END: u64 = MAINNET_AT_10_12_2025_AT_09_00_UTC;
static MAINNET_3RD_PARTY_OFF: LazyLock<FeatureActivation> =
    LazyLock::new(|| {
        FeatureActivation::Ranges(vec![(
            MAINNET_DISABLED_3D_PARTY_START,
            MAINNET_DISABLED_3D_PARTY_END,
        )])
    });
static MAINNET_DISABLE_WASM_64: LazyLock<FeatureActivation> =
    LazyLock::new(|| {
        FeatureActivation::Ranges(vec![(
            MAINNET_DISABLED_3D_PARTY_START,
            u64::MAX,
        )])
    });

const MAINNET_BLOB_ACTIVATION: FeatureActivation =
    FeatureActivation::Height(MAINNET_AT_10_12_2025_AT_09_00_UTC);

/// Mainnet VM configuration.
static MAINNET_CONFIG: LazyLock<WellKnownConfig> =
    LazyLock::new(|| WellKnownConfig {
        gas_per_blob: 0,
        gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
        min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
        min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
        block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
        features: [
            (FEATURE_ABI_PUBLIC_SENDER, MAINNET_SENDER_ACTIVATION_HEIGHT),
            (HQ_KECCAK256, NEVER),
            (FEATURE_BLOB, MAINNET_BLOB_ACTIVATION),
            (FEATURE_DISABLE_WASM64, MAINNET_DISABLE_WASM_64.clone()),
            (FEATURE_DISABLE_WASM32, MAINNET_3RD_PARTY_OFF.clone()),
            (FEATURE_DISABLE_3RD_PARTY, MAINNET_3RD_PARTY_OFF.clone()),
        ],
    });

/// Estimated testnet block height for 12th November 2025, 09:00 UTC.
const TESTNET_AT_12_11_2025_AT_09_00_UTC: FeatureActivation =
    FeatureActivation::Height(1_814_090);

/// Testnet VM configuration.
const TESTNET_CONFIG: WellKnownConfig = WellKnownConfig {
    gas_per_blob: DEFAULT_GAS_PER_BLOB,
    gas_per_deploy_byte: DEFAULT_GAS_PER_DEPLOY_BYTE,
    min_deploy_points: DEFAULT_MIN_DEPLOY_POINTS,
    min_deployment_gas_price: DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
    block_gas_limit: DEFAULT_BLOCK_GAS_LIMIT,
    features: [
        (FEATURE_ABI_PUBLIC_SENDER, GENESIS),
        (HQ_KECCAK256, NEVER),
        (FEATURE_BLOB, TESTNET_AT_12_11_2025_AT_09_00_UTC),
        (FEATURE_DISABLE_WASM64, TESTNET_AT_12_11_2025_AT_09_00_UTC),
        (FEATURE_DISABLE_WASM32, NEVER),
        (FEATURE_DISABLE_3RD_PARTY, NEVER),
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
        (FEATURE_ABI_PUBLIC_SENDER, GENESIS),
        (HQ_KECCAK256, GENESIS),
        (FEATURE_BLOB, GENESIS),
        (FEATURE_DISABLE_WASM64, GENESIS),
        (FEATURE_DISABLE_WASM32, NEVER),
        (FEATURE_DISABLE_3RD_PARTY, NEVER),
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
        (FEATURE_ABI_PUBLIC_SENDER, GENESIS),
        (HQ_KECCAK256, GENESIS),
        (FEATURE_BLOB, GENESIS),
        (FEATURE_DISABLE_WASM64, GENESIS),
        (FEATURE_DISABLE_WASM32, NEVER),
        (FEATURE_DISABLE_3RD_PARTY, NEVER),
    ],
};
