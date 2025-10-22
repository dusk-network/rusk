// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::node::{DriverStore, RuskVmConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, ContractData, Session, VM};
use rusk::{Error, Result, Rusk};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::DEFAULT_MIN_GAS_LIMIT;
use crate::common::wallet::{
    test_wallet as wallet, test_wallet::Wallet, TestStateClient, TestStore,
};

const POINT_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const BOB_INIT_VALUE: u8 = 5;

const CHAIN_ID: u8 = 0xFA;

async fn initial_state<P: AsRef<Path>>(
    dir: P,
    owner: impl AsRef<[u8]>,
) -> Result<Rusk> {
    let dir = dir.as_ref();

    let snapshot =
        toml::from_str(include_str!("../config/contract_deployment.toml"))
            .expect("Cannot deserialize config");

    let dusk_key = *rusk::DUSK_CONSENSUS_KEY;
    let deploy = state::deploy(dir, &snapshot, dusk_key, |session| {
        let bob_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
        );

        session
            .deploy(
                bob_bytecode,
                ContractData::builder()
                    .owner(owner.as_ref())
                    .init_arg(&BOB_INIT_VALUE)
                    .contract_id(gen_contract_id(bob_bytecode, 0u64, owner)),
                POINT_LIMIT,
            )
            .expect("Deploying the bob contract should succeed");
    })
    .expect("Deploying initial state should succeed");

    let (_vm, _commit_id) = deploy;

    let (sender, _) = broadcast::channel(10);

    #[cfg(feature = "archive")]
    let archive_dir =
        tempdir().expect("Should be able to create temporary directory");
    #[cfg(feature = "archive")]
    let archive =
        node::archive::Archive::create_or_open(archive_dir.path()).await;

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        RuskVmConfig::new(),
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

#[allow(dead_code)]
struct Fixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub host_fn_bytecode: Vec<u8>,
    pub bob_bytecode: Vec<u8>,
    pub contract_id: ContractId,
    pub path: PathBuf,
    pub session: Option<Session>,
}

impl Fixture {
    async fn build(owner: impl AsRef<[u8]>) -> Self {
        let tmp =
            tempdir().expect("Should be able to create temporary directory");
        let rusk = initial_state(&tmp, owner.as_ref())
            .await
            .expect("Initializing should succeed");

        let cache = Arc::new(RwLock::new(HashMap::new()));

        let wallet = wallet::Wallet::new(
            TestStore,
            TestStateClient {
                rusk: rusk.clone(),
                cache,
            },
        );

        let original_root = rusk.state_root();

        info!("Original Root: {:?}", hex::encode(original_root));

        let bob_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
        )
        .to_vec();
        let contract_id = gen_contract_id(&bob_bytecode, 0u64, owner.as_ref());

        let path = tmp.into_path();

        let host_fn_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/host_fn.wasm"
        )
        .to_vec();

        Self {
            rusk,
            wallet,
            host_fn_bytecode,
            bob_bytecode,
            contract_id,
            path,
            session: None,
        }
    }

    pub fn assert_bob_contract_is_deployed(&self) {
        const BOB_ECHO_VALUE: u64 = 775;
        let commit = self.rusk.state_root();
        let vm =
            VM::new(self.path.as_path()).expect("VM creation should succeed");
        let mut session = vm
            .session(commit, CHAIN_ID, 0)
            .expect("Session creation should succeed");
        let result = session.call::<_, u64>(
            self.contract_id,
            "echo",
            &BOB_ECHO_VALUE,
            u64::MAX,
        );
        assert_eq!(
            result.expect("Echo call should succeed").data,
            BOB_ECHO_VALUE
        );
        let result =
            session.call::<_, u8>(self.contract_id, "value", &(), u64::MAX);
        assert_eq!(
            result.expect("Value call should succeed").data,
            BOB_INIT_VALUE
        );
    }

    #[allow(dead_code)]
    pub fn set_session(&mut self) {
        let commit = self.rusk.state_root();
        self.set_session_with_commit(&commit)
    }

    pub fn set_session_with_commit(&mut self, commit: &[u8; 32]) {
        let vm =
            VM::new(self.path.as_path()).expect("VM creation should succeed");
        self.session = Some(
            vm.session(*commit, CHAIN_ID, 0)
                .expect("Session creation should succeed"),
        );
    }

    fn assert_bob_contract_call_works(&mut self) {
        let result = self.session.as_mut().unwrap().call::<_, u8>(
            self.contract_id,
            "value",
            &(),
            u64::MAX,
        );
        assert_eq!(
            result.expect("Value call should succeed").data,
            BOB_INIT_VALUE
        );
    }

    fn assert_bob_contract_call_fails(&mut self) {
        let result = self.session.as_mut().unwrap().call::<_, u8>(
            self.contract_id,
            "value",
            &(),
            u64::MAX,
        );
        assert!(result.is_err())
    }

    fn assert_host_fn_contract_call_works(&mut self) {
        let result = self.session.as_mut().unwrap().call::<_, u8>(
            self.contract_id,
            "chain_id",
            &(),
            u64::MAX,
        );
        assert_eq!(result.expect("Ping call should succeed").data, CHAIN_ID);
    }

    fn assert_host_fn_contract_call_fails(&mut self) {
        let result = self.session.as_mut().unwrap().call::<_, u8>(
            self.contract_id,
            "chain_id",
            &(),
            u64::MAX,
        );
        assert!(result.is_err())
    }

    fn contract_self_id(
        &mut self,
        contract_id: &ContractId,
    ) -> Option<ContractId> {
        self.session
            .as_mut()
            .unwrap()
            .contract_metadata(contract_id)
            .map(|metadata| metadata.contract_id)
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn migrate_contract_same_id() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    f.assert_bob_contract_is_deployed();
    let root = f.rusk.state_root();
    f.set_session_with_commit(&root);
    f.assert_bob_contract_call_works();

    // migrate Bob contract to HostFn contract under Bob's contract id
    // note that this is a session-consuming call
    let old_contract = f.contract_id;
    let new_session = f.session.unwrap().migrate(
        old_contract,
        &f.host_fn_bytecode,
        ContractData::builder().owner(NON_BLS_OWNER),
        // .contract_id(old_contract),  //note that setting contract_id to the
        // "old" contract would cause exception
        POINT_LIMIT,
        |new_contract, session| {
            let bob_value = session
                .call::<_, u8>(old_contract, "value", &(), POINT_LIMIT)?
                .data;
            let keccak_input = vec![bob_value];
            session.call::<_, [u8; 32]>(
                new_contract,
                "keccak256",
                &keccak_input,
                POINT_LIMIT,
            )?;
            Ok(())
        },
    )?;

    // we need to check if HostFn works now in a new session under Bob's
    // contract id
    f.session = Some(new_session);
    f.assert_host_fn_contract_call_works();
    // make sure that migrated contract's self id is correct
    let contract_id = f.contract_id;
    assert_eq!(f.contract_self_id(&contract_id), Some(contract_id));

    // make sure the old contract under this is is gone
    f.assert_bob_contract_call_fails();

    // revert the state and see if Bob works again and HostFn fails
    f.set_session_with_commit(&root);
    f.assert_bob_contract_call_works();
    f.assert_host_fn_contract_call_fails();

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn migrate_contract_different_id() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    f.assert_bob_contract_is_deployed();
    let root = f.rusk.state_root();
    f.set_session_with_commit(&root);
    f.assert_bob_contract_call_works();

    const NEW_CONTRACT_ID: ContractId = ContractId::from_bytes([0x78u8; 32]);

    // migrate Bob contract to HostFn contract under Bob's contract id
    // note that this is a session-consuming call
    let old_contract = f.contract_id;
    let new_session = f.session.unwrap().migrate(
        old_contract,
        &f.host_fn_bytecode,
        ContractData::builder()
            .owner(NON_BLS_OWNER)
            // note: this only decides the id during the migration
            // after migration the id will be unchanged (i.e., equal
            // 'old_contract') anyway
            .contract_id(NEW_CONTRACT_ID),
        POINT_LIMIT,
        |new_contract, session| {
            let bob_value = session
                .call::<_, u8>(old_contract, "value", &(), POINT_LIMIT)?
                .data;
            let keccak_input = vec![bob_value];
            session.call::<_, [u8; 32]>(
                new_contract,
                "keccak256",
                &keccak_input,
                POINT_LIMIT,
            )?;
            Ok(())
        },
    )?;

    // we need to check if HostFn works now in a new session under Bob's
    // contract id
    f.session = Some(new_session);
    f.assert_host_fn_contract_call_works();
    // make sure that migrated contract's self id is correct
    let contract_id = f.contract_id;
    assert_eq!(f.contract_self_id(&contract_id), Some(contract_id));

    // make sure the old contract under this is is gone
    f.assert_bob_contract_call_fails();

    // revert the state and see if Bob works again and HostFn fails
    f.set_session_with_commit(&root);
    f.assert_bob_contract_call_works();
    f.assert_host_fn_contract_call_fails();

    Ok(())
}
