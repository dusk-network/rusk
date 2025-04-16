// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC VM Adapter
//!
//! This module provides an abstraction layer for interacting with the node's
//! Virtual Machine (VM) component. It defines the [`VmAdapter`] trait, which
//! specifies the VM operations required by the JSON-RPC service, such as
//! transaction simulation, preverification, and querying VM state (e.g.,
//! provisioners, state root, gas limits).
//!
//! The primary implementation, [`RuskVmAdapter`], wraps the actual
//! `node::vm::VMExecution` implementation (feature-gated behind `chain`). This
//! adapter pattern isolates the JSON-RPC layer from the core VM logic,
//! enhancing testability (e.g., using `MockVmAdapter` from test utilities)
//! and maintainability.
//!
//! Errors related to VM operations are defined in [`VmError`].

use crate::jsonrpc::infrastructure::error::VmError;
use crate::jsonrpc::model::{
    provisioner::{ProvisionerInfo, StakeInfo},
    transaction::SimulationResult,
};
use async_trait::async_trait;
use node::vm::PreverificationResult;
use std::fmt::{self, Debug};

// Imports specific to RuskVmAdapter (require 'chain' feature)
#[cfg(feature = "chain")]
use {
    crate::jsonrpc::model::provisioner::StakeOwnerInfo,
    dusk_bytes::DeserializableSlice,
    dusk_consensus::user::{provisioners::Provisioners, stake::Stake},
    hex,
    node::vm::VMExecution,
    node_data::ledger::Transaction,
    node_data::Serializable, // For Transaction::read
    std::sync::Arc,
    tokio::sync::RwLock,
    tokio::task,
};

/// Trait defining the interface for VM operations needed by the JSON-RPC
/// service.
///
/// This trait abstracts the interaction with the underlying node's VM execution
/// components, providing methods to simulate transactions, query state, and get
/// consensus-related information managed by the VM. Implementations of this
/// trait wrap the actual VM client (like `node::vm::VMExecution`).
#[async_trait]
pub trait VmAdapter: Send + Sync + Debug + 'static {
    /// Simulates the execution of a transaction without applying state changes.
    ///
    /// This is useful for estimating gas costs or predicting the outcome of a
    /// transaction before broadcasting it.
    /// Corresponds to the underlying VM simulation logic.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to be simulated.
    ///
    /// # Returns
    ///
    /// * `Ok(SimulationResult)` - Contains details about the simulation outcome
    ///   (e.g., gas used, return value, logs).
    /// * `Err(VmError)` - If the simulation failed (e.g., invalid transaction,
    ///   execution error, internal VM error).
    async fn simulate_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<SimulationResult, VmError>;

    /// Performs preverification checks on a transaction.
    ///
    /// Corresponds to `node::vm::VMExecution::preverify`.
    /// Checks performed may include signature validation, nonce checks, and
    /// basic structural validity without full execution.
    ///
    /// # Arguments
    ///
    /// * `tx_bytes` - The serialized transaction bytes to preverify.
    ///
    /// # Returns
    ///
    /// * `Ok(PreverificationResult)` - Indicates whether the preverification
    ///   checks passed or failed, potentially with details.
    /// * `Err(VmError)` - If the preverification process encountered an
    ///   internal error.
    async fn preverify_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<PreverificationResult, VmError>;

    /// Retrieves the current set of active provisioners known by the VM.
    ///
    /// Corresponds to `node::vm::VMExecution::get_provisioners`.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ProvisionerInfo>)` - A list containing information about each
    ///   active provisioner.
    /// * `Err(VmError)` - If retrieving the provisioner set failed.
    async fn get_provisioners(&self) -> Result<Vec<ProvisionerInfo>, VmError>;

    /// Retrieves detailed staking information for a specific provisioner.
    ///
    /// Corresponds to `node::vm::VMExecution::get_provisioner` (or similar
    /// logic).
    ///
    /// # Arguments
    ///
    /// * `public_key_bls_hex` - The hex-encoded BLS public key of the
    ///   provisioner.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(StakeInfo))` - Contains detailed staking information if the
    ///   provisioner is found.
    /// * `Ok(None)` - If no provisioner with the given public key is found.
    /// * `Err(VmError)` - If querying the stake information failed (e.g.,
    ///   invalid public key format, internal VM error).
    async fn get_stake_info(
        &self,
        public_key_bls_hex: &str,
    ) -> Result<Option<StakeInfo>, VmError>;

    /// Retrieves the current state root hash from the VM.
    ///
    /// Corresponds to `node::vm::VMExecution::get_state_root`.
    ///
    /// # Returns
    ///
    /// * `Ok([u8; 32])` - The 32-byte state root hash.
    /// * `Err(VmError)` - If retrieving the state root failed.
    async fn get_state_root(&self) -> Result<[u8; 32], VmError>;

    /// Retrieves the gas limit for a block from the VM.
    ///
    /// Corresponds to `node::vm::VMExecution::get_block_gas_limit`.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The block gas limit.
    /// * `Err(VmError)` - If retrieving the gas limit failed.
    async fn get_block_gas_limit(&self) -> Result<u64, VmError>;
}

/// Real implementation of the `VmAdapter` for the Rusk node.
///
/// This struct wraps an `Arc<RwLock<VM>>` where `VM` implements
/// `node::vm::VMExecution`, allowing interaction with the actual VM component.
#[cfg(feature = "chain")]
pub struct RuskVmAdapter<VM: VMExecution> {
    /// Shared, thread-safe access to the VM client.
    vm_client: Arc<RwLock<VM>>,
}

#[cfg(feature = "chain")]
impl<VM: VMExecution> RuskVmAdapter<VM> {
    /// Creates a new `RuskVmAdapter`.
    ///
    /// # Arguments
    ///
    /// * `vm_client` - An `Arc<RwLock<VM>>` pointing to the node's VM
    ///   component.
    pub fn new(vm_client: Arc<RwLock<VM>>) -> Self {
        Self { vm_client }
    }

    // Helper function to convert PublicKey and Stake into
    // ProvisionerInfo/StakeInfo
    //
    // TODO: This conversion is incomplete as
    // `consensus::user::stake::Stake` only contains `value` and
    // `eligibility`. Missing fields (locked_amount, reward, faults,
    // hard_faults, owner) are defaulted. The `VMExecution` trait
    // needs modification to return richer stake info (like StakeData)
    // to fully populate this model.
    fn stake_to_info(
        pk: &node_data::bls::PublicKey,
        stake: &Stake,
    ) -> ProvisionerInfo {
        ProvisionerInfo {
            public_key: pk.to_base58(),
            amount: stake.value(),
            eligibility: stake.eligible_since, /* TODO: Confirm if this
                                                * mapping is correct for the
                                                * model's definition of
                                                * eligibility */
            // Defaulted / Missing fields:
            locked_amount: 0, /* Not available in
                               * `consensus::user::stake::Stake` */
            reward: 0,      // Not available
            faults: 0,      // Not available
            hard_faults: 0, // Not available
            owner: StakeOwnerInfo::Account(String::new()), /* Placeholder -
                             * owner type/
                             * address not
                             * available */
        }
    }
}

// Manual Debug implementation to avoid requiring VM: Debug and potentially
// leaking sensitive info.
#[cfg(feature = "chain")]
impl<VM: VMExecution> fmt::Debug for RuskVmAdapter<VM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuskVmAdapter")
            .field("vm_client", &"Arc<RwLock<VM: VMExecution>>")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "chain")]
#[async_trait]
impl<VM: VMExecution> VmAdapter for RuskVmAdapter<VM> {
    async fn simulate_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<SimulationResult, VmError> {
        // TODO: Implement simulation. This likely involves:
        // 1. Getting a temporary VM session/state.
        // 2. Deserializing tx_bytes into a `Transaction`.
        // 3. Calling `dusk_vm::execute` or a similar function on the session.
        // 4. Mapping the `CallReceipt` (gas_spent, data: Result<...>) to
        //    `SimulationResult`.
        // This might require `spawn_blocking` if VM execution is CPU-intensive.
        // Returning placeholder for now.
        Err(VmError::InternalError(
            "simulate_transaction not yet implemented".to_string(),
        ))
    }

    async fn preverify_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<PreverificationResult, VmError> {
        let tx = Transaction::read(&mut tx_bytes.as_slice()).map_err(|e| {
            VmError::QueryFailed(format!(
                "Failed to deserialize transaction: {}",
                e
            ))
        })?;

        // VM calls are potentially CPU-intensive, use spawn_blocking
        let client = self.vm_client.clone();
        let result = task::spawn_blocking(move || {
            let guard = client.blocking_read(); // Use blocking read inside blocking task
            guard.preverify(&tx)
        })
        .await
        .map_err(|e| VmError::InternalError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            VmError::InternalError(format!("VM preverify error: {}", e))
        })?; // Explicitly map anyhow::Error

        Ok(result)
    }

    async fn get_provisioners(&self) -> Result<Vec<ProvisionerInfo>, VmError> {
        // VM calls are potentially CPU-intensive, use spawn_blocking
        let client = self.vm_client.clone();
        let provisioners_map: Provisioners = task::spawn_blocking(move || {
            // Need the current state root for get_provisioners
            let guard = client.blocking_read();
            let state_root = guard.get_state_root()?; // Use blocking read
            guard.get_provisioners(state_root)
        })
        .await
        .map_err(|e| VmError::InternalError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            VmError::InternalError(format!("VM get_provisioners error: {}", e))
        })?; // Explicitly map anyhow::Error

        // Convert the BTreeMap from Provisioners into Vec<ProvisionerInfo>
        let info_vec = provisioners_map
            .iter()
            .map(|(pk, stake)| Self::stake_to_info(pk, stake))
            .collect();

        Ok(info_vec)
    }

    async fn get_stake_info(
        &self,
        public_key_bls_hex: &str,
    ) -> Result<Option<StakeInfo>, VmError> {
        // Decode hex public key
        let pk_bytes = hex::decode(public_key_bls_hex).map_err(|e| {
            VmError::QueryFailed(format!("Invalid hex public key: {}", e))
        })?;
        let bls_pk =
            dusk_core::signatures::bls::PublicKey::from_slice(&pk_bytes)
                .map_err(|e| {
                    VmError::QueryFailed(format!(
                        "Invalid BLS public key: {:?}",
                        e
                    )) // Use debug format {:?}
                })?;
        let node_pk = node_data::bls::PublicKey::new(bls_pk);

        // VM calls are potentially CPU-intensive, use spawn_blocking
        let client = self.vm_client.clone();
        let stake_option: Option<Stake> = task::spawn_blocking(move || {
            let guard = client.blocking_read(); // Use blocking read
            guard.get_provisioner(&bls_pk)
        })
        .await
        .map_err(|e| VmError::InternalError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            VmError::InternalError(format!("VM get_provisioner error: {}", e))
        })?; // Explicitly map anyhow::Error

        // Convert Option<Stake> to Option<StakeInfo>
        let info_option =
            stake_option.map(|stake| Self::stake_to_info(&node_pk, &stake));

        Ok(info_option)
    }

    async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
        // VM calls are potentially CPU-intensive, use spawn_blocking
        let client = self.vm_client.clone();
        let root = task::spawn_blocking(move || {
            let guard = client.blocking_read(); // Use blocking read
            guard.get_state_root()
        })
        .await
        .map_err(|e| VmError::InternalError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            VmError::InternalError(format!("VM get_state_root error: {}", e))
        })?; // Explicitly map anyhow::Error
        Ok(root)
    }

    async fn get_block_gas_limit(&self) -> Result<u64, VmError> {
        // This is likely just reading a config value, probably not blocking.
        // However, keeping consistent with spawn_blocking for VM interactions.
        let client = self.vm_client.clone();
        let limit = task::spawn_blocking(move || {
            let guard = client.blocking_read(); // Use blocking read
            guard.get_block_gas_limit()
        })
        .await
        .map_err(|e| {
            VmError::InternalError(format!("Task join error: {}", e))
        })?;
        // ^ Note: No inner error mapping needed as get_block_gas_limit returns
        // u64 directly.
        Ok(limit)
    }
}
