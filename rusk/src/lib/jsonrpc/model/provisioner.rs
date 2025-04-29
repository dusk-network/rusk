// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Provisioner and Staking Models
//!
//! This module defines the data structures used to represent information about
//! network provisioners (stakers) and their associated stakes within the
//! JSON-RPC API.
//!
//! Provisioners are nodes that participate in the network consensus by staking
//! DUSK tokens. These models provide a standardized format for querying and
//! displaying details about provisioners, their stake amounts, eligibility,
//! rewards, and potential faults.
//!
//! ## Key Structures:
//!
//! - [`ProvisionerInfo`]: Provides comprehensive details about a single
//!   provisioner, including their public key, various stake amounts (total,
//!   locked, eligible), rewards, fault counts, and owner information.
//! - [`StakeOwnerInfo`]: An enum distinguishing whether a stake is owned by a
//!   regular user account (identified by a BLS public key) or a smart contract
//!   (identified by a contract ID).
//! - [`StakeInfo`]: A type alias for `ProvisionerInfo`, commonly used in
//!   contexts where the focus is on the stake itself rather than the
//!   provisioner node role.

use serde::{Deserialize, Serialize};

/// Represents information about the owner of a stake.
///
/// A stake in the Dusk network can be initiated either directly from a user's
/// wallet account or through a staking smart contract. This enum distinguishes
/// between these two ownership types.
///
/// It uses `serde` attributes for JSON serialization: `tag = "type"` creates a
/// field named `type` containing the variant name (`"Account"` or
/// `"Contract"`), and `content = "address"` puts the associated value (public
/// key or contract ID) into a field named `address`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "address", rename_all = "PascalCase")]
pub enum StakeOwnerInfo {
    /// Indicates the stake is owned directly by a user account.
    /// The associated `String` contains the Base58-encoded BLS public key of
    /// the owner account.
    Account(String),
    /// Indicates the stake is managed by a smart contract.
    /// The associated `String` contains the hex-encoded contract ID of the
    /// managing contract.
    Contract(String),
}

/// Represents detailed information about a single network provisioner and their
/// stake.
///
/// This structure aggregates key data points relevant to a provisioner's status
/// and performance within the consensus mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // Use camelCase for JSON field names
pub struct ProvisionerInfo {
    /// The unique identifier of the provisioner node.
    /// This is the Base58-encoded BLS public key associated with the node.
    pub public_key: String,
    /// The total amount of DUSK staked by this provisioner.
    /// Represented in atomic units (e.g., 1 DUSK = 10^8 atomic units).
    /// Serialized as a numeric string to avoid precision issues with large
    /// numbers in JSON.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub amount: u64,
    /// The portion of the total stake that is currently locked (e.g., due to
    /// unstaking periods or slashing conditions).
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub locked_amount: u64,
    /// The amount of the stake currently eligible to participate in consensus
    /// rounds.
    /// This might differ from the total `amount` due to locking or other
    /// factors. Represented in atomic units and serialized as a numeric
    /// string. Often referred to as the "active" stake amount.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub eligibility: u64,
    /// The total amount of rewards accumulated by this provisioner but not yet
    /// claimed or compounded.
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub reward: u64,
    /// The number of minor consensus faults (e.g., missed block proposals/
    /// votes) recorded for this provisioner.
    pub faults: u8,
    /// The number of severe consensus faults (hard faults) recorded,
    /// potentially leading to more significant slashing.
    pub hard_faults: u8,
    /// Information identifying the owner (account or contract) of this stake.
    /// See [`StakeOwnerInfo`].
    pub owner: StakeOwnerInfo,
}

/// Type alias for [`ProvisionerInfo`].
///
/// This alias is often used in API methods or contexts where the primary focus
/// is querying information about a specific stake, even though the underlying
/// data structure includes all provisioner details.
pub type StakeInfo = ProvisionerInfo;
