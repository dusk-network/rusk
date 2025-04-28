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
//! The primary implementation, [`RuskVmAdapter`], wraps the main Rusk node
//! logic (`node::Rusk`), which orchestrates VM interactions. This
//! adapter pattern isolates the JSON-RPC layer from the core node logic,
//! enhancing testability (e.g., using `MockVmAdapter` from test utilities)
//! and maintainability.
//!
//! Errors related to VM operations are defined in [`VmError`].
//!
//! For a detailed method comparison vs. the legacy HTTP server, see:
//! [`VM Adapter Methods Comparison`](../../../../docs/vm_adapter_methods.md)

use crate::node::Rusk as NodeRusk;
use crate::node::RuskVmConfig;
use async_trait::async_trait;
use dusk_consensus::user::stake::Stake;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use node::vm::PreverificationResult;
use node_data::ledger::Transaction;
use node_data::Serializable as NodeSerializable;
use std::fmt::{self, Debug};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::jsonrpc::infrastructure::error::VmError;
use crate::jsonrpc::model::provisioner::{ProvisionerInfo, StakeOwnerInfo};
use crate::jsonrpc::model::transaction::SimulationResult;
use dusk_bytes::Serializable;
use dusk_core::stake::{StakeAmount, StakeData, StakeFundOwner, StakeKeys};

use dusk_vm::execute;
use node::vm::VMExecution;

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

    /// Retrieves the current chain ID from the VM.
    ///
    /// # Required Method
    ///
    /// Corresponds to `node::Rusk::chain_id`.
    ///
    /// # Returns
    ///
    /// * `Ok(u8)` - The chain ID.
    /// * `Err(VmError)` - If retrieving the chain ID failed.
    async fn get_chain_id(&self) -> Result<u8, VmError>;

    /// Retrieves account data (balance and nonce) for a given public key.
    ///
    /// # Required Method
    ///
    /// Corresponds to `node::Rusk::account`.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(AccountData)` - The account's balance and nonce.
    /// * `Err(VmError)` - If the account query failed (e.g., account not found,
    ///   internal error).
    async fn get_account_data(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, VmError>;

    /// Retrieves the balance for a given account public key.
    ///
    /// # Default Method
    ///
    /// This method has a default implementation that uses
    /// [`get_account_data`](VmAdapter::get_account_data).
    /// Implementors of `VmAdapter` only need to provide `get_account_data`.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The account's balance.
    /// * `Err(VmError)` - If the underlying query failed.
    async fn get_account_balance(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<u64, VmError> {
        Ok(self.get_account_data(pk).await?.balance)
    }

    /// Retrieves the nonce for a given account public key.
    ///
    /// # Default Method
    ///
    /// This method has a default implementation that uses
    /// [`get_account_data`](VmAdapter::get_account_data).
    /// Implementors of `VmAdapter` only need to provide `get_account_data`.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the account to query.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The account's nonce.
    /// * `Err(VmError)` - If the underlying query failed.
    async fn get_account_nonce(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<u64, VmError> {
        Ok(self.get_account_data(pk).await?.nonce)
    }

    /// Retrieves the current state root hash from the VM.
    ///
    /// # Required Method
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
    /// # Required Method
    ///
    /// Corresponds to `node::vm::VMExecution::get_block_gas_limit`.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The block gas limit.
    /// * `Err(VmError)` - If retrieving the gas limit failed.
    async fn get_block_gas_limit(&self) -> Result<u64, VmError>;

    /// Retrieves the full details (StakeKeys, StakeData) for all current
    /// provisioners from the VM state.
    ///
    /// # Required Method
    /// Corresponds to `node::Rusk::provisioners`.
    /// Requires the current state root internally.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(StakeKeys, StakeData)>)` - A vector containing tuples of
    ///   stake keys and stake data for each provisioner.
    /// * `Err(VmError)` - If retrieving the provisioners failed.
    async fn get_provisioners(
        &self,
    ) -> Result<Vec<(StakeKeys, StakeData)>, VmError>;

    /// Retrieves stake information for a single provisioner by their BLS public
    /// key.
    ///
    /// # Required Method
    ///
    /// Corresponds to `node::vm::VMExecution::get_provisioner`.
    ///
    /// # Arguments
    ///
    /// * `pk` - The BLS public key of the provisioner.
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Stake>)` - The stake information if the provisioner exists,
    ///   otherwise `None`.
    /// * `Err(VmError)` - If the query failed.
    async fn get_stake_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<Stake>, VmError>;

    /// Retrieves the stake data for a specific provisioner by their public key.
    ///
    /// # Default Method - Now potentially incorrect if get_stake_info_by_pk changes/is removed
    async fn get_stake_data_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<Stake>, VmError> {
        // Same underlying logic as get_stake_info_by_pk
        self.get_stake_info_by_pk(pk).await
    }

    /// Retrieves a list of all provisioners and their corresponding simplified
    /// stake data (`Stake`).
    ///
    /// This default implementation calls `get_provisioners` and maps the
    /// detailed `(StakeKeys, StakeData)` pairs into `(BlsPublicKey, Stake)`
    /// pairs. If a provisioner has no stake amount (`StakeData.amount` is
    /// `None`), a default `Stake::new(0, 0)` is used.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(BlsPublicKey, Stake)>)` - A vector containing tuples of BLS
    ///   public keys and their simplified stake information.
    /// * `Err(VmError)` - If retrieving the provisioners failed.
    async fn get_all_stake_data(
        &self,
    ) -> Result<Vec<(BlsPublicKey, Stake)>, VmError> {
        // Retrieve full provisioners details
        let provisioners_details = self.get_provisioners().await?;

        // Map the detailed data to the simplified (PublicKey, Stake) format
        let data = provisioners_details
            .into_iter()
            .map(|(keys, data)| {
                // Extract value and eligibility from StakeData.amount,
                // defaulting to 0 if None.
                let (value, eligibility) =
                    data.amount.map_or((0, 0), |sa| (sa.value, sa.eligibility));
                // Create the simplified Stake struct
                let consensus_stake = Stake::new(value, eligibility);
                (keys.account, consensus_stake) // Use the account key directly
            })
            .collect();

        Ok(data)
    }

    /// Executes a read-only query on a contract at a specific state commit.
    ///
    /// # Required Method
    ///
    /// # Arguments
    /// * `contract_id` - The ID of the contract to query.
    /// * `method` - The name of the contract method to call.
    /// * `base_commit` - The state commit hash to execute the query against.
    /// * `args_bytes` - The serialized arguments for the contract method.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The serialized result bytes from the contract query.
    /// * `Err(VmError)` - If the query failed.
    async fn query_contract_raw(
        &self,
        contract_id: dusk_core::abi::ContractId,
        method: String,
        base_commit: [u8; 32],
        args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, VmError>;

    /// Retrieves the VM configuration settings.
    ///
    /// # Required Method
    async fn get_vm_config(&self) -> Result<RuskVmConfig, VmError>;

    /// Retrieves detailed information about a single provisioner by public key.
    ///
    /// This default implementation filters the results from `get_provisioners`.
    async fn get_provisioner_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<ProvisionerInfo, VmError> {
        let all_details = self.get_provisioners().await?; // Call the refactored method

        // Find the details for the requested public key
        if let Some((keys, data)) =
            all_details.into_iter().find(|(k, _)| k.account == *pk)
        {
            // Map the found StakeKeys and StakeData to the ProvisionerInfo
            // model
            let pk_b58 = bs58::encode(keys.account.to_bytes()).into_string();

            // Extract amount details, providing defaults if None
            let (amount, locked, eligibility) =
                data.amount.map_or((0, 0, 0), |sa: StakeAmount| {
                    (sa.value, sa.locked, sa.eligibility)
                });

            // Map StakeFundOwner to StakeOwnerInfo
            let owner_info = match keys.owner {
                StakeFundOwner::Account(owner_pk) => StakeOwnerInfo::Account(
                    bs58::encode(owner_pk.to_bytes()).into_string(),
                ),
                StakeFundOwner::Contract(contract_id) => {
                    StakeOwnerInfo::Contract(hex::encode(
                        contract_id.to_bytes(),
                    ))
                }
            };

            Ok(ProvisionerInfo {
                public_key: pk_b58,
                amount,
                locked_amount: locked,
                eligibility,
                reward: data.reward,
                faults: data.faults,
                hard_faults: data.hard_faults,
                owner: owner_info,
            })
        } else {
            // Provisioner not found in the list
            Err(VmError::QueryFailed(format!(
                "Provisioner details not found for public key: {}",
                bs58::encode(pk.to_bytes()).into_string()
            )))
        }
    }
}

/// Real implementation of the `VmAdapter` for the Rusk node.
///
/// This struct wraps an `Arc<tokio::sync::RwLock<NodeRusk>>` allowing
/// interaction with the main Rusk node logic, ensuring thread-safe access.
#[cfg(feature = "chain")]
#[derive(Clone)]
pub struct RuskVmAdapter {
    /// Shared, lock-protected access to the main Rusk node instance.
    node_rusk_lock: Arc<RwLock<NodeRusk>>,
}

#[cfg(feature = "chain")]
impl RuskVmAdapter {
    /// Creates a new `RuskVmAdapter`.
    ///
    /// # Arguments
    ///
    /// * `node_rusk_lock` - An `Arc<tokio::sync::RwLock<NodeRusk>>` pointing to
    ///   the main Rusk node instance, typically obtained from
    ///   `node.inner().vm_handler()`.
    pub fn new(node_rusk_lock: Arc<RwLock<NodeRusk>>) -> Self {
        Self { node_rusk_lock }
    }
}

// Manual Debug implementation
#[cfg(feature = "chain")]
impl fmt::Debug for RuskVmAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuskVmAdapter")
            .field("node_rusk_lock", &"Arc<tokio::sync::RwLock<node::Rusk>>")
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "chain")]
#[async_trait]
impl VmAdapter for RuskVmAdapter {
    /// In our JSON-RPC VM adapter, `simulate_transaction` provides
    /// a pure VM preview — gas estimates, return values, and logs — without
    /// touching on-disk state or invoking consensus/mempool logic.
    ///
    /// We use a fixed, dummy block height of 0 (with the current in-memory
    /// state root) to achieve:
    /// 1. **Isolation from consensus state**: we don't read from the database,
    ///    aren't blocked on loading the tip, and never mutate or commit any
    ///    on-chain state. Simulation becomes a self-contained VM execution that
    ///    can be run entirely in memory.
    /// 2. **Determinism & reproducibility**: height-dependent features (e.g.
    ///    activating "public sender" once you cross a certain block) won't
    ///    accidentally flip on or off mid-chain. At height 0 everything is in
    ///    its "genesis" configuration, so you get consistent results every
    ///    time.
    /// 3. **Simplicity & performance**: no extra I/O or expensive DB lookups,
    ///    no need to spawn an async block to fetch the tip, and no risk of
    ///    races. The code lives entirely in our VM API, wrapped in one
    ///    `spawn_blocking` call, which is much lighter than forking off a full
    ///    block session against the live node state.
    /// 4. **Testability**: simulations run entirely in memory, requiring no
    ///    full consensus node or populated chain data. If you wanted the
    ///    simulation to more closely mirror an on-chain "dry-run" (with real
    ///    tip height, mempool gas checks, feature flags, etc.), you'd have to
    ///    pull in the DB, load the tip, guard the gas limit, and potentially
    ///    risk side-effects or slower performance. By using a dummy height we
    ///    strike a clean, efficient balance: clients get a fast,
    ///    side-effect-free VM preview.
    async fn simulate_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<SimulationResult, VmError> {
        let tx = Transaction::read(&mut tx_bytes.as_slice()).map_err(|e| {
            VmError::QueryFailed(format!(
                "Failed to deserialize transaction: {}",
                e
            ))
        })?;

        // Clone the lock Arc to move into the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();

        tokio::task::spawn_blocking(move || {
            // Acquire lock synchronously inside the blocking task
            let node_guard = node_rusk_lock.blocking_read();
            let base_commit = node_guard.state_root();

            // Initialize a VM session using the guard
            let mut session = node_guard
                .new_block_session(0, base_commit)
                .map_err(|e| VmError::ExecutionFailed(e.to_string()))?;
            let config = node_guard.vm_config.to_execution_config(0);
            let receipt = execute(&mut session, &tx.inner, &config);
            // Map to SimulationResult
            let sim = match receipt {
                Ok(receipt) => SimulationResult {
                    success: true,
                    gas_estimate: Some(receipt.gas_spent),
                    error: None,
                },
                Err(err) => SimulationResult {
                    success: false,
                    gas_estimate: None,
                    error: Some(format!("{:?}", err)),
                },
            };
            Ok(sim)
        })
        .await
        .map_err(|e| VmError::InternalError(e.to_string()))?
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

        // Clone the lock Arc to move into the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();

        tokio::task::spawn_blocking(move || {
            // Acquire lock synchronously inside the blocking task
            let node_guard = node_rusk_lock.blocking_read();
            // Call the original method on the guard
            node_guard
                .preverify(&tx)
                .map_err(|e| VmError::QueryFailed(e.to_string()))
        })
        .await
        .map_err(|e| VmError::InternalError(e.to_string()))?
    }

    /// Retrieves the current chain ID from the Rusk node.
    ///
    /// Spawns a blocking task to delegate the call to `node::Rusk::chain_id`.
    async fn get_chain_id(&self) -> Result<u8, VmError> {
        // Acquire read lock asynchronously for quick read
        let node_guard = self.node_rusk_lock.read().await;
        // Delegate to the underlying method (assuming it's quick)
        node_guard
            .chain_id()
            .map_err(|e| VmError::QueryFailed(e.to_string()))
    }

    /// Retrieves account data (balance and nonce) for a given public key from
    /// the Rusk node.
    ///
    /// Spawns a blocking task to delegate the call to `node::Rusk::account`.
    async fn get_account_data(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, VmError> {
        // Clone the lock Arc and key for the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();
        let key = *pk; // Copy pk so it can be moved

        tokio::task::spawn_blocking(move || {
            // Acquire lock synchronously inside the blocking task
            let node_guard = node_rusk_lock.blocking_read();
            // Call the original method on the guard
            node_guard
                .account(&key)
                .map_err(|e| VmError::QueryFailed(e.to_string()))
        })
        .await
        .map_err(|e| VmError::InternalError(e.to_string()))?
    }

    async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
        // Acquire read lock asynchronously for quick read
        Ok(self.node_rusk_lock.read().await.state_root())
    }

    /// Retrieves the gas limit for a block directly from the Rusk node's
    /// config.
    async fn get_block_gas_limit(&self) -> Result<u64, VmError> {
        // Acquire read lock asynchronously for quick read
        Ok(self.node_rusk_lock.read().await.vm_config.block_gas_limit)
    }

    /// Retrieves the full details (StakeKeys, StakeData) for all current
    /// provisioners from the Rusk node.
    ///
    /// Spawns a blocking task to delegate the call to
    /// `node::Rusk::provisioners`.
    async fn get_provisioners(
        &self,
    ) -> Result<Vec<(StakeKeys, StakeData)>, VmError> {
        // Clone the lock Arc for the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();

        tokio::task::spawn_blocking(
            move || -> Result<Vec<(StakeKeys, StakeData)>, String> {
                // Acquire lock synchronously inside the blocking task
                let node_guard = node_rusk_lock.blocking_read();
                let provisioner_iter =
                    node_guard.provisioners(None).map_err(|e| {
                        format!("Failed to get provisioners iterator: {}", e)
                    })?;
                let details: Vec<(StakeKeys, StakeData)> =
                    provisioner_iter.collect();
                Ok(details)
            },
        )
        .await
        .map_err(|e| VmError::InternalError(format!("Task join error: {}", e)))?
        .map_err(VmError::QueryFailed)
    }

    /// Retrieves stake information for a single provisioner by their BLS public
    /// key.
    async fn get_stake_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<Stake>, VmError> {
        // Clone the lock Arc and key for the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();
        let key = *pk;

        tokio::task::spawn_blocking(move || {
            // Acquire lock synchronously inside the blocking task
            let node_guard = node_rusk_lock.blocking_read();
            node_guard
                .provisioner(&key)
                .map(|stake_data_option| {
                    stake_data_option.map(|stake_data| {
                        let amount = stake_data.amount.unwrap_or_default();
                        Stake::new(amount.value, amount.eligibility)
                    })
                })
                .map_err(|e| VmError::QueryFailed(e.to_string()))
        })
        .await
        .map_err(|e| VmError::InternalError(e.to_string()))?
    }

    /// Executes a read-only query on a contract at a specific state commit
    /// using the Rusk node's VM session.
    ///
    /// Spawns a blocking task to perform the query.
    async fn query_contract_raw(
        &self,
        contract_id: dusk_core::abi::ContractId,
        method: String,
        base_commit: [u8; 32],
        args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, VmError> {
        // Clone the lock Arc for the blocking task
        let node_rusk_lock = self.node_rusk_lock.clone();

        tokio::task::spawn_blocking(move || {
            // Acquire lock synchronously inside the blocking task
            let node_guard = node_rusk_lock.blocking_read();
            // Create a session at the specified base commit using the guard
            let mut session = node_guard
                .query_session(Some(base_commit))
                .map_err(|e| VmError::QueryFailed(e.to_string()))?;
            let receipt = session
                .call_raw(
                    contract_id,
                    method.as_ref(),
                    args_bytes,
                    node_guard.vm_config.block_gas_limit, // Use guard here too
                )
                .map_err(|e| VmError::QueryFailed(e.to_string()))?;
            Ok(receipt.data)
        })
        .await
        .map_err(|e| VmError::InternalError(e.to_string()))?
    }

    /// Retrieves the VM configuration settings directly from the Rusk node's
    /// config.
    async fn get_vm_config(&self) -> Result<RuskVmConfig, VmError> {
        // Acquire read lock asynchronously for quick read
        Ok(self.node_rusk_lock.read().await.vm_config.clone())
    }
}
