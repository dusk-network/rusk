// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phf::Map;
use phf_macros::phf_map;

pub(crate) const MAINNET: u8 = 1;
pub(crate) const TESTNET: u8 = 2;
pub(crate) const DEVNET: u8 = 3;

// MAINNET constants
pub(crate) const MAINNET_GAS_PER_BLOB: u64 = 0;
pub(crate) const MAINNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
pub(crate) const MAINNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
pub(crate) const MAINNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
pub(crate) const MAINNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
pub(crate) const MAINNET_GENERATION_TIMEOUT: u64 = 3;
pub(crate) static MAINNET_FEATURES: Map<&'static str, u64> = phf_map! {
    // "feature1" => 1u64 /*activation height*/,
    // "feature2" => 2u64 /*activation height*/,
};

// TESTNET constants
pub(crate) const TESTNET_GAS_PER_BLOB: u64 = 0;
pub(crate) const TESTNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
pub(crate) const TESTNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
pub(crate) const TESTNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
pub(crate) const TESTNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
pub(crate) const TESTNET_GENERATION_TIMEOUT: u64 = 3;
pub(crate) static TESTNET_FEATURES: Map<&'static str, u64> = phf_map! {
    // "feature1" => 1u64 /*activation height*/,
    // "feature2" => 2u64 /*activation height*/,
};

// DEVNET constants
pub(crate) const DEVNET_GAS_PER_BLOB: u64 = 0;
pub(crate) const DEVNET_GAS_PER_DEPLOY_BYTE: u64 = 100;
pub(crate) const DEVNET_MIN_DEPLOY_POINTS: u64 = 5_000_000;
pub(crate) const DEVNET_MIN_DEPLOYMENT_GAS_PRICE: u64 = 2_000;
pub(crate) const DEVNET_BLOCK_GAS_LIMIT: u64 = 3_000_000_000;
pub(crate) const DEVNET_GENERATION_TIMEOUT: u64 = 3;
pub(crate) static DEVNET_FEATURES: Map<&'static str, u64> = phf_map! {
    // "feature1" => 1u64 /*activation height*/,
    // "feature2" => 2u64 /*activation height*/,
};
