// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub use anyhow::Result;
use common::{
    state::new_state,
    wallet::{self, DummyCacheItem, TestStateClient, TestStore, Wallet},
};
pub use rusk::node::RuskVmConfig;
pub use rusk::{Result as RuskResult, Rusk};
use tempfile::tempdir;

pub mod common;

/// This struct contains the common setup for the tests, including the Rusk
/// instance and a test wallet. It also contains the temporary directory used
/// for the Rusk state, which will be automatically cleaned up when the struct
/// is dropped.
pub struct TestContext {
    _temp_dir: tempfile::TempDir,
    wallet: common::wallet::Wallet<TestStore, TestStateClient>,
    rusk: Rusk,
    wallet_cache: Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>>,
}

impl TestContext {
    /// Creates a new `TestContext` with the given state configuration and VM
    /// configuration. This function sets up the Rusk instance and the test
    /// wallet, and returns the context for use in tests.
    pub async fn instantiate(
        state_toml: &str,
        vm_config: RuskVmConfig,
    ) -> anyhow::Result<Self> {
        let _temp_dir = tempdir().map_err(|e| {
            anyhow::anyhow!("Should be able to create temporary directory: {e}")
        })?;

        let snapshot = toml::from_str(state_toml)
            .map_err(|e| anyhow::anyhow!("Cannot deserialize config: {e}"))?;

        let rusk = new_state(_temp_dir.path(), &snapshot, vm_config).await?;

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
        })
    }

    pub fn rusk(&self) -> &Rusk {
        &self.rusk
    }

    pub fn wallet(&self) -> &Wallet<TestStore, TestStateClient> {
        &self.wallet
    }

    pub fn wallet_cache(
        &self,
    ) -> Arc<RwLock<HashMap<Vec<u8>, DummyCacheItem>>> {
        self.wallet_cache.clone()
    }

    pub fn state_root(&self) -> [u8; 32] {
        self.rusk.state_root()
    }
}
