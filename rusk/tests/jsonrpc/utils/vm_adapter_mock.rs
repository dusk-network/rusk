// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;

use rusk::jsonrpc::infrastructure::{error::VmError, vm::VmAdapter};
use rusk::jsonrpc::model;

use std::collections::HashMap;

use dusk_bytes::Serializable;

/// Mock implementation of `VmAdapter` for testing.
#[derive(Default)]
pub struct MockVmAdapter {
    /// Force an error on all method calls if Some.
    pub force_error: Option<VmError>,
    /// Predefined simulation result.
    pub simulation_result: Option<model::transaction::SimulationResult>,
    /// Predefined preverification result.
    pub preverification_result: Option<model::vm::VmPreverificationResult>,
    /// Predefined list of provisioners with model types.
    pub provisioners: Option<
        Vec<(
            model::provisioner::ProvisionerKeys,
            model::provisioner::ProvisionerStakeData,
        )>,
    >,
    /// Predefined stake info map (BLS pubkey -> ConsensusStakeInfo).
    pub stakes: Option<HashMap<String, model::provisioner::ConsensusStakeInfo>>,
    /// Predefined state root.
    pub state_root: Option<[u8; 32]>,
    /// Predefined block gas limit.
    pub block_gas_limit: Option<u64>,
    /// Predefined chain ID.
    pub chain_id: Option<u8>,
    /// Predefined AccountData map for get_account_data. Use Vec as
    /// BlsPublicKey doesn't impl Ord or Hash.
    /// Stores model::account::AccountInfo now.
    pub account_data: Option<Vec<(BlsPublicKey, model::account::AccountInfo)>>,
    /// Predefined VmConfig
    pub vm_config: Option<model::vm::VmConfig>,
    /// Set of existing nullifiers for the mock.
    pub existing_nullifiers_set: Option<std::collections::HashSet<[u8; 32]>>,
}

// Manual implementation of Debug
impl std::fmt::Debug for MockVmAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockVmAdapter")
            .field("force_error", &self.force_error)
            .field("simulation_result", &self.simulation_result)
            .field("preverification_result", &self.preverification_result)
            .field("provisioners", &self.provisioners)
            .field("stakes", &self.stakes)
            .field("state_root", &self.state_root)
            .field("block_gas_limit", &self.block_gas_limit)
            .field("chain_id", &self.chain_id)
            .field("account_data", &self.account_data)
            .field("vm_config", &self.vm_config)
            .field("existing_nullifiers_set", &self.existing_nullifiers_set)
            .finish()
    }
}

// Manual implementation of Clone
impl Clone for MockVmAdapter {
    fn clone(&self) -> Self {
        Self {
            force_error: self.force_error.clone(),
            simulation_result: self.simulation_result.clone(),
            preverification_result: self.preverification_result.clone(),
            provisioners: self.provisioners.clone(),
            stakes: self.stakes.clone(),
            state_root: self.state_root,
            block_gas_limit: self.block_gas_limit,
            chain_id: self.chain_id,
            account_data: self.account_data.clone(),
            vm_config: self.vm_config.clone(),
            existing_nullifiers_set: self.existing_nullifiers_set.clone(),
        }
    }
}

#[async_trait::async_trait]
impl VmAdapter for MockVmAdapter {
    async fn simulate_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<model::transaction::SimulationResult, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        self.simulation_result.clone().ok_or_else(|| {
            VmError::InternalError("Mock simulation result not set".to_string())
        })
    }

    async fn preverify_transaction(
        &self,
        _tx_bytes: Vec<u8>,
    ) -> Result<model::vm::VmPreverificationResult, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Return the predefined result or default to Valid
        Ok(self
            .preverification_result
            .clone()
            .unwrap_or(model::vm::VmPreverificationResult::Valid))
    }

    async fn get_chain_id(&self) -> Result<u8, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.chain_id.unwrap_or(0)) // Default mock value
    }

    async fn get_account_data(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<model::account::AccountInfo, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Iterate through Vec to find the key
        if let Some(vec) = &self.account_data {
            for (key, data) in vec {
                if key == pk {
                    return Ok(data.clone());
                }
            }
        }
        // Default if not found in vec or vec is None
        Ok(model::account::AccountInfo {
            balance: 0,
            nonce: 0,
        })
    }

    async fn get_state_root(&self) -> Result<[u8; 32], VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.state_root.unwrap_or([0u8; 32])) // Default mock value
    }

    async fn get_block_gas_limit(&self) -> Result<u64, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(self.block_gas_limit.unwrap_or(1_000_000_000)) // Default high limit
    }

    async fn get_provisioners(
        &self,
    ) -> Result<
        Vec<(
            model::provisioner::ProvisionerKeys,
            model::provisioner::ProvisionerStakeData,
        )>,
        VmError,
    > {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Return predefined or empty Vec
        Ok(self.provisioners.clone().unwrap_or_default())
    }

    async fn get_stake_info_by_pk(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<model::provisioner::ConsensusStakeInfo>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        // Look up in the stakes map if provided
        if let Some(stakes_map) = &self.stakes {
            // Use Serializable::to_bytes()
            let pk_bytes = pk.to_bytes();
            let pk_b58 = bs58::encode(pk_bytes).into_string();
            Ok(stakes_map.get(&pk_b58).cloned())
        } else {
            Ok(None) // Default mock implementation: None
        }
    }

    async fn query_contract_raw(
        &self,
        _contract_id: dusk_core::abi::ContractId,
        _method: String,
        _base_commit: [u8; 32],
        _args_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        Ok(Vec::new()) // Default mock: empty result
    }

    async fn get_vm_config(&self) -> Result<model::vm::VmConfig, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }
        let mut features = HashMap::with_capacity(1);
        features.insert("ABI_PUBLIC_SENDER".to_string(), 1000000);

        // Return a predefined config or a default config for the mock
        Ok(model::vm::VmConfig {
            block_gas_limit: 3000000000,
            gas_per_deploy_byte: 100,
            min_deploy_points: 5000000,
            min_deployment_gas_price: 2000,
            generation_timeout: Some(std::time::Duration::from_secs(2)),
            features,
        })
    }

    async fn validate_nullifiers(
        &self,
        nullifiers: &[[u8; 32]],
    ) -> Result<Vec<[u8; 32]>, VmError> {
        if let Some(err) = &self.force_error {
            return Err(err.clone());
        }

        if let Some(existing_set) = &self.existing_nullifiers_set {
            let spent_nullifiers = nullifiers
                .iter()
                .filter(|n| existing_set.contains(*n))
                .cloned()
                .collect();
            Ok(spent_nullifiers)
        } else {
            // Default: no nullifiers exist if set is not provided
            Ok(Vec::new())
        }
    }
}
