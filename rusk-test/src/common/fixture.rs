// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::{Path, PathBuf};

use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, Session};
use rusk::node::{
    driverstore::DriverStore, RuskVmConfig, FEATURE_ABI_PUBLIC_SENDER,
};
use rusk::{Result, Rusk};
use rusk_recovery_tools::state::restore_state;
use tempfile::TempDir;
use tokio::sync::broadcast;

#[cfg(feature = "archive")]
use node::archive::Archive;
#[cfg(feature = "archive")]
use tempfile::tempdir;

use crate::common::state::DEFAULT_MIN_GAS_LIMIT;
use crate::common::wallet::{TestContext, TestStateClient, TestStore, Wallet};

pub struct DeployFixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub bob_bytecode: Vec<u8>,
    pub contract_id: ContractId,
    pub path: PathBuf,
    pub session: Option<Session>,
}

impl DeployFixture {
    pub fn new(tmp: TempDir, rusk: Rusk, owner: impl AsRef<[u8]>) -> Self {
        let path = tmp.into_path();
        let TestContext {
            rusk,
            wallet,
            cache: _,
            original_root: _,
        } = TestContext::new(rusk);

        let bob_bytecode =
            include_bytes!("../../../contracts/bin/bob.wasm").to_vec();
        let contract_id = gen_contract_id(&bob_bytecode, 0u64, owner.as_ref());

        Self {
            rusk,
            wallet,
            bob_bytecode,
            contract_id,
            path,
            session: None,
        }
    }
}

pub struct StateFixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    // Keeps the temporary state directory alive for the lifetime of the
    // fixture.
    #[allow(dead_code)]
    pub tmpdir: TempDir,
}

impl StateFixture {
    pub async fn build(state_archive: &[u8], chain_id: u8) -> Self {
        let tmpdir: TempDir = tempfile::tempdir().expect("tempdir() to work");
        let state_dir = tmpdir.path().join("state");

        rusk_recovery_tools::state::tar::unarchive(
            state_archive,
            state_dir.as_path(),
        )
        .expect("unarchive should work");

        let rusk = rusk_from_restored_state(&state_dir, chain_id)
            .await
            .expect("Initializing should succeed");

        let TestContext {
            rusk,
            wallet,
            cache: _,
            original_root: _,
        } = TestContext::new(rusk);

        Self {
            rusk,
            wallet,
            tmpdir,
        }
    }
}

async fn rusk_from_restored_state(dir: &Path, chain_id: u8) -> Result<Rusk> {
    let (_vm, _commit_id) = restore_state(dir)?;

    let (sender, _) = broadcast::channel(10);

    #[cfg(feature = "archive")]
    let archive_dir =
        tempdir().expect("Should be able to create temporary directory");
    #[cfg(feature = "archive")]
    let archive = Archive::create_or_open(archive_dir.path()).await;

    let mut vm_config =
        RuskVmConfig::new().with_block_gas_limit(10_000_000_000);
    vm_config.with_feature(FEATURE_ABI_PUBLIC_SENDER, 1);

    let rusk = Rusk::new(
        dir,
        chain_id,
        vm_config,
        DEFAULT_MIN_GAS_LIMIT,
        u64::MAX,
        sender,
        #[cfg(feature = "archive")]
        archive,
        DriverStore::new(None::<PathBuf>),
    )
    .expect("Instantiating rusk should succeed");
    Ok(rusk)
}
