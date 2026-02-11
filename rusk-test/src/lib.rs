// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub use anyhow::Result;
pub use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
pub use dusk_core::transfer::Transaction;
pub use node_data::ledger::SpentTransaction;
pub use rusk::node::RuskVmConfig;
pub use rusk::{Result as RuskResult, Rusk};
pub use rusk_recovery_tools::state::Session;
use tempfile::tempdir;

use common::{
    state::{
        generator_procedure, generator_procedure2, new_state_with,
        ExecuteResult, LOCAL_TEST_CHAIN_ID,
    },
    wallet::{self, DummyCacheItem, TestStateClient, TestStore, Wallet},
};

/// This struct contains the common setup for the tests, including the Rusk
/// instance and a test wallet. It also contains the temporary directory used
/// for the Rusk state, which will be automatically cleaned up when the struct
/// is dropped.
pub struct TestContext {
    _temp_dir: tempfile::TempDir,
    wallet: common::wallet::Wallet<TestStore, TestStateClient>,
    rusk: Rusk,
    wallet_cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
    vm_config: RuskVmConfig,
}

impl TestContext {
    /// Creates a new `TestContext` with the given state configuration and VM
    /// configuration. This function sets up the Rusk instance and the test
    /// wallet, and returns the context for use in tests.
    pub async fn instantiate(
        state_toml: &str,
        vm_config: RuskVmConfig,
    ) -> anyhow::Result<Self> {
        Self::instantiate_with(state_toml, vm_config, |_| {}).await
    }

    /// Creates a new `TestContext` with the given state configuration and VM
    /// configuration. This function sets up the Rusk instance and the test
    /// wallet, and returns the context for use in tests.
    pub async fn instantiate_with<F>(
        state_toml: &str,
        vm_config: RuskVmConfig,
        closure: F,
    ) -> anyhow::Result<Self>
    where
        F: FnOnce(&mut Session),
    {
        let _temp_dir = tempdir().map_err(|e| {
            anyhow::anyhow!("Should be able to create temporary directory: {e}")
        })?;

        let snapshot = toml::from_str(state_toml)
            .map_err(|e| anyhow::anyhow!("Cannot deserialize config: {e}"))?;

        let rusk = new_state_with(
            _temp_dir.path(),
            &snapshot,
            vm_config.clone(),
            LOCAL_TEST_CHAIN_ID,
            closure,
        )
        .await?;

        let wallet_cache = Arc::new(RwLock::new(HashMap::new()));

        // Create a wallet
        let wallet = wallet::Wallet::new(
            TestStore,
            TestStateClient {
                rusk: rusk.clone(),
                cache: wallet_cache.clone(),
            },
        );

        Ok(TestContext {
            _temp_dir,
            wallet,
            rusk,
            wallet_cache,
            vm_config,
        })
    }

    pub fn rusk(&self) -> &Rusk {
        &self.rusk
    }

    pub fn wallet(&self) -> &Wallet<TestStore, TestStateClient> {
        &self.wallet
    }

    pub fn state_root(&self) -> [u8; 32] {
        self.rusk.state_root()
    }

    /// Executes a transaction and asserts that the output error matches the
    /// expected error.
    ///
    /// # Arguments
    ///
    /// * `tx` - The transaction to execute.
    /// * `block_height` - The block height at which to execute the transaction.
    /// * `expected_error` - The expected error message, or `None` if no error
    ///   is expected.
    /// # Returns
    ///
    /// The `SpentTransaction` resulting from the execution of the transaction.
    ///
    /// # Panics
    ///
    /// This function will panic if the generator procedure fails, if the
    /// transaction is not executed, or if the output error does not match the
    /// expected error.
    ///
    /// Note: for more granular assertions on the transaction output, consider
    /// using the `generator_procedure` function directly in the test, which
    /// allows you to inspect the full execution result.
    pub fn execute_transaction<'a, E: Into<Option<&'a str>>>(
        &self,
        tx: Transaction,
        block_height: u64,
        expected_error: E,
    ) -> SpentTransaction {
        let executed_txs = generator_procedure(
            &self.rusk,
            &[tx],
            block_height,
            self.vm_config.block_gas_limit,
            vec![],
            Some(ExecuteResult {
                executed: 1,
                discarded: 0,
            }),
        )
        .expect("generator procedure to succeed");
        let tx = executed_txs
            .into_iter()
            .next()
            .expect("Transaction must be executed");

        let tx_error = tx.err.as_deref();
        let error = expected_error.into();
        assert_eq!(tx_error, error, "Output error does not match");
        tx
    }

    pub fn slash(
        &self,
        block_height: u64,
        to_slash: Vec<BlsPublicKey>,
    ) -> Result<()> {
        generator_procedure(&self.rusk, &[], block_height, 0, to_slash, None)?;
        Ok(())
    }

    pub fn empty_block(&self, block_height: u64) -> Result<[u8; 32]> {
        let (_, root) = generator_procedure2(
            &self.rusk,
            &[],
            block_height,
            self.vm_config.block_gas_limit,
            vec![],
            None,
            None,
        )?;
        Ok(root)
    }

    pub fn revert_to_base_root(&self) -> Result<()> {
        self.rusk
            .revert_to_base_root()
            .map_err(|e| anyhow::anyhow!("Cannot revert to base root {e}"))?;
        self.wallet_cache
            .write()
            .map_err(|e| anyhow::anyhow!("Cannot clear wallet cache {e}"))?
            .clear();
        Ok(())
    }
}
