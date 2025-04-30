// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC VM Models
//!
//! This module contains models related to Virtual Machine (VM) information
//! used in JSON-RPC responses, such as preverification results
//! ([`VmPreverificationResult`]) and VM configuration ([`VmConfig`]).

use crate::jsonrpc::model::account::AccountInfo;
use crate::jsonrpc::model::key::AccountPublicKey;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents the result of a transaction preverification check performed by
/// the VM.
///
/// This enum indicates whether a transaction passed basic checks (like
/// signature, nonce validity relative to current state) before full execution
/// or inclusion in the mempool.
///
/// It mirrors the structure of `node::vm::PreverificationResult` but uses
/// JSON-RPC specific models like `AccountPublicKey` and `AccountInfo` for
/// consistent serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VmPreverificationResult {
    /// Indicates the transaction passed all preverification checks
    /// successfully.
    Valid,
    /// Indicates the transaction failed preverification because its nonce is
    /// higher than the expected next nonce for the sender's account.
    FutureNonce {
        /// The public key of the account whose nonce check failed.
        /// Boxed to reduce the memory footprint of the enum.
        /// Serialized as Base58.
        account: Box<AccountPublicKey>,
        /// The current state (nonce and balance) of the account at the time of
        /// the check.
        state: AccountInfo,
        /// The nonce value provided in the transaction that was deemed too
        /// high.
        nonce_used: u64,
    },
}

/// Converts the internal `node::vm::PreverificationResult` enum to the
/// JSON-RPC `VmPreverificationResult` model.
impl From<node::vm::PreverificationResult> for VmPreverificationResult {
    fn from(result: node::vm::PreverificationResult) -> Self {
        match result {
            node::vm::PreverificationResult::Valid => {
                VmPreverificationResult::Valid
            }
            node::vm::PreverificationResult::FutureNonce {
                account,
                state,
                nonce_used,
            } => VmPreverificationResult::FutureNonce {
                // Convert internal BlsPublicKey to model AccountPublicKey and
                // Box it
                account: Box::new(AccountPublicKey(account)),
                // Convert internal AccountData to model AccountInfo via From
                state: AccountInfo::from(state),
                nonce_used,
            },
        }
    }
}

/// Represents the relevant VM configuration settings exposed via JSON-RPC.
///
/// This provides information about gas limits, deployment costs, and other
/// parameters influencing transaction execution and block generation within the
/// VM.
///
/// Mirrors a subset of fields from `crate::node::vm::Config` (aliased as
/// `RuskVmConfig`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmConfig {
    /// The cost in gas points charged for each byte of a smart contract's
    /// deployment bytecode.
    /// Helps determine the base cost of deploying contracts.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub gas_per_deploy_byte: u64,
    /// The minimum baseline gas points charged for any contract deployment,
    /// regardless of bytecode size.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub min_deploy_points: u64,
    /// The minimum gas price (gas points per gas unit) required for a
    /// transaction that deploys a contract.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub min_deployment_gas_price: u64,
    /// The maximum total gas points that can be consumed by all transactions
    /// included within a single block.
    /// Acts as a ceiling on block complexity and execution time.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub block_gas_limit: u64,
    /// The configured timeout duration for the process of generating a
    /// candidate block.
    /// If set, this limits how long the node will spend attempting to
    /// construct a block before giving up.
    /// Serialized as an optional human-readable string (e.g., "5s", "1m")
    /// using `humantime_serde::option`.
    #[serde(with = "humantime_serde::option", default)]
    pub generation_timeout: Option<Duration>,
}

/// Converts the internal `RuskVmConfig` (alias for `crate::node::vm::Config`)
/// into the JSON-RPC `VmConfig` model.
impl From<crate::node::RuskVmConfig> for VmConfig {
    fn from(config: crate::node::RuskVmConfig) -> Self {
        VmConfig {
            gas_per_deploy_byte: config.gas_per_deploy_byte,
            min_deploy_points: config.min_deploy_points,
            min_deployment_gas_price: config.min_deployment_gas_price,
            block_gas_limit: config.block_gas_limit,
            generation_timeout: config.generation_timeout,
            // Note: `features` field from the internal config is intentionally
            // omitted from the JSON-RPC model.
        }
    }
}
