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
//! - [`ProvisionerStakeAmount`]: Represents the breakdown of a stake's value,
//!   locked amount, and eligibility height. Mirrors
//!   `dusk_core::stake::StakeAmount`.
//! - [`ProvisionerStakeData`]: Aggregates stake amount details with rewards and
//!   fault counts. Mirrors `dusk_core::stake::StakeData`.
//! - [`ProvisionerKeys`]: Represents the keys (account and owner) associated
//!   with a stake. Mirrors `dusk_core::stake::StakeKeys`.
//! - [`ConsensusStakeInfo`]: Provides simplified stake information (value and
//!   eligibility) used in consensus contexts. Mirrors
//!   `dusk_consensus::user::stake::Stake`.

use dusk_bytes::Serializable;
use serde::{Deserialize, Serialize};

use crate::jsonrpc::model::key::AccountPublicKey;

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

/// Represents the amounts and eligibility associated with a provisioner's
/// stake.
///
/// Mirrors `dusk_core::stake::StakeAmount` for JSON-RPC, providing a detailed
/// breakdown of the stake's financial state and consensus readiness.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerStakeAmount {
    /// The portion of the stake that is currently active and contributing to
    /// consensus weight, excluding any locked amounts.
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub value: u64,
    /// The portion of the stake that is currently locked and cannot be
    /// withdrawn or used for consensus immediately (e.g., due to soft
    /// slashes or unstaking periods).
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub locked: u64,
    /// The block height at which this specific stake amount (or the most
    /// recent change to it) becomes eligible to participate in the
    /// consensus mechanism. Staking operations typically have a maturity
    /// period before the stake becomes active.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub eligibility: u64,
}

impl From<dusk_core::stake::StakeAmount> for ProvisionerStakeAmount {
    fn from(amount: dusk_core::stake::StakeAmount) -> Self {
        ProvisionerStakeAmount {
            value: amount.value,
            locked: amount.locked,
            eligibility: amount.eligibility,
        }
    }
}

/// Represents the overall stake data for a provisioner, including amounts,
/// rewards, and fault counts.
///
/// Mirrors `dusk_core::stake::StakeData` for JSON-RPC, aggregating key staking
/// metrics.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerStakeData {
    /// The stake amount details (`value`, `locked`, `eligibility`), if the
    /// provisioner has an active stake. This field is flattened in JSON,
    /// meaning its subfields appear directly within this structure if present.
    /// If the provisioner has no stake, these fields will be absent.
    /// See [`ProvisionerStakeAmount`].
    #[serde(flatten, default)]
    pub amount: Option<ProvisionerStakeAmount>,
    /// The total accumulated rewards earned by the provisioner that have not
    /// yet been withdrawn or compounded into the stake.
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub reward: u64,
    /// The number of minor consensus faults (e.g., missed votes or proposals)
    /// recorded against this provisioner. These might lead to warnings or
    /// soft slashes.
    pub faults: u8,
    /// The number of severe consensus faults (hard faults) recorded against
    /// this provisioner. These typically result in more significant
    /// penalties, such as hard slashing.
    pub hard_faults: u8,
}

impl From<dusk_core::stake::StakeData> for ProvisionerStakeData {
    fn from(data: dusk_core::stake::StakeData) -> Self {
        ProvisionerStakeData {
            amount: data.amount.map(ProvisionerStakeAmount::from),
            reward: data.reward,
            faults: data.faults,
            hard_faults: data.hard_faults,
        }
    }
}

/// Represents the keys associated with a provisioner's stake: the account key
/// for consensus and the owner key/contract for funds.
///
/// Mirrors `dusk_core::stake::StakeKeys` for JSON-RPC, using appropriate model
/// types like `AccountPublicKey` and `StakeOwnerInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerKeys {
    /// The BLS public key used by the provisioner for consensus operations
    /// like signing blocks or votes.
    /// Serialized as Base58 using the `AccountPublicKey` wrapper.
    pub account: AccountPublicKey,
    /// Information identifying the owner of the staked funds. This is
    /// flattened in JSON to include the `type` (Account/Contract) and
    /// `address` (Base58 key / Hex contract ID) fields directly.
    /// See [`StakeOwnerInfo`].
    #[serde(flatten)]
    pub owner: StakeOwnerInfo,
}

impl From<dusk_core::stake::StakeKeys> for ProvisionerKeys {
    fn from(keys: dusk_core::stake::StakeKeys) -> Self {
        let owner_info = match keys.owner {
            dusk_core::stake::StakeFundOwner::Account(pk) => {
                let pk_bytes = pk.to_bytes();
                let pk_b58 = bs58::encode(pk_bytes).into_string();
                StakeOwnerInfo::Account(pk_b58)
            }
            dusk_core::stake::StakeFundOwner::Contract(id) => {
                StakeOwnerInfo::Contract(hex::encode(id.to_bytes()))
            }
        };

        ProvisionerKeys {
            account: AccountPublicKey(keys.account),
            owner: owner_info,
        }
    }
}

/// Represents simplified stake information (value and eligibility) primarily
/// used within consensus logic contexts.
///
/// Mirrors `dusk_consensus::user::stake::Stake` for JSON-RPC, providing just
/// the essential details needed for consensus participation checks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusStakeInfo {
    /// The value of the stake considered for consensus weight.
    /// Represented in atomic units and serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub value: u64,
    /// The block height from which this stake is considered eligible to
    /// participate in the consensus process.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub eligible_since: u64,
}

impl From<dusk_consensus::user::stake::Stake> for ConsensusStakeInfo {
    fn from(stake: dusk_consensus::user::stake::Stake) -> Self {
        ConsensusStakeInfo {
            value: stake.value(), // Use getter for private field
            eligible_since: stake.eligible_since,
        }
    }
}
