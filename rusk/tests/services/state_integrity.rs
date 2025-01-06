// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, ContractData, Session, VM};
use rusk::{Error, Result, Rusk};
use rusk_recovery_tools::state;
// use tempfile::tempdir;
use test_wallet::{self as wallet, Wallet};
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::state::DEFAULT_MIN_DEPLOYMENT_GAS_PRICE;
use crate::common::state::DEFAULT_MIN_DEPLOY_POINTS;
use crate::common::state::{
    DEFAULT_GAS_PER_DEPLOY_BYTE, DEFAULT_MIN_GAS_LIMIT,
};
use crate::common::wallet::{TestStateClient, TestStore};

const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const POINT_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const BOB_INIT_VALUE: u8 = 5;

const METHOD: &str = "reset";

const CHAIN_ID: u8 = 0xFA;

fn initial_state<P: AsRef<Path>>(
    dir: P,
    owner: impl AsRef<[u8]>,
) -> Result<Rusk> {
    let dir = dir.as_ref();

    let snapshot =
        toml::from_str(include_str!("../config/contract_deployment.toml"))
            .expect("Cannot deserialize config");

    let (_vm, _commit_id) = state::deploy(dir, &snapshot, |session| {
        let bob_bytecode = include_bytes!(
            "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
        );

        session
            .deploy(
                bob_bytecode,
                ContractData::builder()
                    .owner(owner.as_ref())
                    .init_arg(&BOB_INIT_VALUE)
                    .contract_id(gen_contract_id(&bob_bytecode, 0u64, owner)),
                POINT_LIMIT,
            )
            .expect("Deploying the bob contract should succeed");
    })
    .expect("Deploying initial state should succeed");

    let (sender, _) = broadcast::channel(10);

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
        None,
        DEFAULT_GAS_PER_DEPLOY_BYTE,
        DEFAULT_MIN_DEPLOYMENT_GAS_PRICE,
        DEFAULT_MIN_GAS_LIMIT,
        DEFAULT_MIN_DEPLOY_POINTS,
        BLOCK_GAS_LIMIT,
        u64::MAX,
        sender,
    )
    .expect("Instantiating rusk should succeed");
    Ok(rusk)
}

#[allow(dead_code)]
struct Fixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub bob_bytecode: Vec<u8>,
    pub contract_id: ContractId,
    pub path: PathBuf,
}

impl Fixture {
    fn build(owner: impl AsRef<[u8]>) -> Self {
        // let tmp =
        //     tempdir().expect("Should be able to create temporary directory");
        let tmp = PathBuf::from("/Users/miloszm/.dusk/rusk/state");
        let rusk = initial_state(&tmp, owner.as_ref())
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

        // let path = tmp.into_path();
        let path = tmp;
        Self {
            rusk,
            wallet,
            bob_bytecode,
            contract_id,
            path,
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

    pub fn create_session(&mut self, vm: &VM) -> Session {
        let commit = self.rusk.state_root();
        vm.session(commit, CHAIN_ID, 0)
            .expect("Session creation should succeed")
    }

    pub fn create_vm(&mut self) -> VM {
        VM::new(self.path.as_path()).expect("VM creation should succeed")
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn make_commits() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER);
    f.assert_bob_contract_is_deployed();
    let vm = f.create_vm();

    for value in 0..10u8 {
        let mut session = f.create_session(&vm);
        let r = session
            .call::<u8, ()>(f.contract_id, METHOD, &value, u64::MAX)
            .map_err(Error::Vm);
        assert!(r.is_ok());
        let root = session.commit()?;
        println!("{}", hex::encode(root));
    }
    Ok(())
}
