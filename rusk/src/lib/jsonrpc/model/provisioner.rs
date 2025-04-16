// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! JSON-RPC Models for Provisioner and Staking Information.

use serde::{Deserialize, Serialize};

/// Represents information about the owner of a stake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "address", rename_all = "PascalCase")]
pub enum StakeOwnerInfo {
    /// Stake is owned by a regular user account.
    /// The value is the Base58-encoded BLS public key.
    Account(String),
    /// Stake is owned by a contract.
    /// The value is the hex-encoded contract ID.
    Contract(String),
}

/// Represents detailed information about a provisioner node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerInfo {
    /// Base58-encoded BLS public key of the provisioner node.
    pub public_key: String,
    /// Staked amount in Dusk atomic units, as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub amount: u64,
    /// Locked stake amount in Dusk atomic units, as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub locked_amount: u64,
    /// Current eligibility score/epoch, as a numeric string.
    /// TODO: Verify exact meaning (epoch? round? block height?).
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub eligibility: u64,
    /// Accumulated rewards in Dusk atomic units, as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub reward: u64,
    /// Number of minor faults recorded.
    pub faults: u8,
    /// Number of severe (hard) faults recorded.
    pub hard_faults: u8,
    /// Information about the owner of the stake.
    pub owner: StakeOwnerInfo,
}

/// Type alias for `ProvisionerInfo`, often used when retrieving information
/// about a single provisioner's stake.
pub type StakeInfo = ProvisionerInfo;
