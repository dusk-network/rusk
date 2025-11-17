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
use rusk_recovery_tools::state::restore_state;

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

    // let snapshot =
    //     toml::from_str(include_str!("../config/contract_deployment.toml"))
    //         .expect("Cannot deserialize config");
    //
    // let dusk_key = *rusk::DUSK_CONSENSUS_KEY;
    // let deploy = state::deploy(dir, &snapshot, dusk_key, |session| {
    //     let bob_bytecode = include_bytes!(
    //         "../../../target/dusk/wasm32-unknown-unknown/release/bob.wasm"
    //     );
    //
    //     session
    //         .deploy(
    //             bob_bytecode,
    //             ContractData::builder()
    //                 .owner(owner.as_ref())
    //                 .init_arg(&BOB_INIT_VALUE)
    //                 .contract_id(gen_contract_id(bob_bytecode, 0u64, owner)),
    //             POINT_LIMIT,
    //         )
    //         .expect("Deploying the bob contract should succeed");
    // })
    // .expect("Deploying initial state should succeed");

    // let (_vm, _commit_id) = deploy;
    let (_vm, _commit_id) = restore_state(dir)?;

    // let s = vm.session(commit_id, 1, 1)?;

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
}

const NEW_FN: &str = "chain_id";
const OLD_FN: &str = "value";

impl Fixture {
    async fn build(owner: impl AsRef<[u8]>) -> Self {
        // let tmp =
        //     tempdir().expect("Should be able to create temporary directory");
        let tmp = PathBuf::from("/Users/miloszm/Downloads/state_11");
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

        //let path = tmp.into_path();

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
        }
    }

    pub fn assert_bob_contract_is_deployed(&self) {
        const BOB_ECHO_VALUE: u64 = 775;
        let mut session = self
            .rusk
            .query_session(None)
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

    fn query_tip(&self, fn_name: &str) -> Result<u8, dusk_vm::Error> {
        let mut session = self
            .rusk
            .query_session(None)
            .expect("Query session should work");

        let result =
            session.call::<_, u8>(self.contract_id, fn_name, &(), u64::MAX)?;
        Ok(result.data)
    }
}

fn migrate_data(
    old_contract: ContractId,
    new_contract: ContractId,
    session: &mut Session,
) -> core::result::Result<(), dusk_vm::Error> {
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
}

#[tokio::test(flavor = "multi_thread")]
pub async fn migrate_contract_same_id() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    let before_migrate = f.rusk.state_root();

    f.assert_bob_contract_is_deployed();
    f.query_tip(OLD_FN).expect("old contract should work");

    // migrate old contract to new contract under old contract id
    // note that this is a session-consuming call
    let old_contract_id = f.contract_id;
    let new_root = {
        let new_session = f.rusk.new_block_session(0, before_migrate).unwrap();
        let new_session = new_session.migrate(
            old_contract_id,
            &f.host_fn_bytecode,
            ContractData::builder()
                .owner(NON_BLS_OWNER)
                .contract_id(ContractId::from_bytes([0x78u8; 32])),
            // note that setting contract_id to the
            // old contract would cause "contract already exists" exception,
            // otherwise, if we set the contract data contract id
            // to a value which does not correspond to any deployed contract,
            // the value will only be used in the migration closure
            // and then discarded
            POINT_LIMIT,
            |new_contract, session| {
                migrate_data(old_contract_id, new_contract, session)
            },
        )?;
        f.rusk.commit_session(new_session)?
    };

    f.query_tip(NEW_FN).expect("new contract should work");
    f.query_tip(OLD_FN).expect_err("old contract should fail");

    // note that id is of the old contract
    // make sure that migrated contract's self id is correct
    let metadata = f
        .rusk
        .query_metadata(&old_contract_id)
        .expect("metadata query should work");
    assert_eq!(metadata.contract_id, old_contract_id);

    // revert the state and see if old contract works again and new contract
    // fails
    f.rusk.revert(before_migrate)?;
    f.query_tip(NEW_FN).expect_err("new contract should fail");
    f.query_tip(OLD_FN).expect("old contract should work");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn migrate_contract_finalization() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    f.assert_bob_contract_is_deployed();
    f.query_tip(OLD_FN).expect("old contract should work");

    let session = f.rusk.new_block_session(0, f.rusk.state_root()).unwrap();

    let old_contract = f.contract_id;
    let new_session = session.migrate(
        old_contract,
        &f.host_fn_bytecode,
        ContractData::builder().owner(NON_BLS_OWNER),
        POINT_LIMIT,
        |new_contract, session| {
            migrate_data(old_contract, new_contract, session)
        },
    )?;

    let to_merge = f.rusk.commit_session(new_session)?;

    // advance by 1
    let s = f.rusk.new_block_session(1, to_merge).unwrap();
    let to_finalize = f.rusk.commit_session(s)?;

    // advance by 1 again
    let s = f.rusk.new_block_session(1, to_finalize).unwrap();
    let tip = f.rusk.commit_session(s)?;

    f.rusk.finalize_state(to_finalize, vec![to_merge])?;

    // move the tip beyond to_finalize
    let _tip_session = f
        .rusk
        .new_block_session(1, tip)
        .expect("tip session should succeed");

    // check that to_finalize is not the tip any more
    let finalized_session = f
        .rusk
        .new_block_session(1, to_finalize)
        .expect_err("finalized session should return an error");
    match finalized_session {
        dusk_consensus::errors::StateTransitionError::TipChanged => {}
        _ => panic!("Expected TipChanged error"),
    }

    // check that to_merge is merged and querying it gives a Vm error
    let merged_session = f
        .rusk
        .query_session(Some(to_merge))
        .expect_err("merged session should return an Error");
    match merged_session {
        Error::Vm(e) => {}
        e => panic!("Expected SessionError error {e}"),
    }

    f.query_tip(NEW_FN).expect("new contract should work");
    f.query_tip(OLD_FN).expect_err("old contract should fail");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn migrate_contract_reversion() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    f.assert_bob_contract_is_deployed();
    let before_migrate = f.rusk.state_root();
    f.query_tip(OLD_FN).expect("old contract should work");

    let session = f.rusk.new_block_session(0, before_migrate).unwrap();

    let old_contract = f.contract_id;
    let new_session = session.migrate(
        old_contract,
        &f.host_fn_bytecode,
        ContractData::builder().owner(NON_BLS_OWNER),
        POINT_LIMIT,
        |new_contract, session| {
            migrate_data(old_contract, new_contract, session)
        },
    )?;

    f.rusk.commit_session(new_session)?;
    let reverted = f.rusk.revert(before_migrate)?;
    let s = f.rusk.new_block_session(1, reverted).unwrap();
    f.rusk.commit_session(s)?;

    f.query_tip(NEW_FN).expect_err("new contract should fail");
    f.query_tip(OLD_FN).expect("old contract should work");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_ab() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    println!("root={}", hex::encode(f.rusk.state_root()));
    let base = hex::decode("6ab0bd65f1b57f6d0efa4bf26b49e11f1e09f1ece008cdcdbe57a044f7b6bdaa").unwrap();
    let mut base_a: [u8; 32] = [0u8; 32];
    base_a.copy_from_slice(&base);
    let mut lock = f.rusk.tip.write();
    lock.current = base_a;
    drop(lock);
    // let root = f.rusk.state_root();
    let mut session = f.rusk.new_block_session(1, base_a).unwrap();

    let tx_vec = hex::decode("01a40400000000000001942fc94c5fed7c4925c27361a552b4290b8a1f1d0584671d19f4cb5ac4d13307c3d87b274bb403f2d153bc9f8dcacf3c010ccd7438df51154f5df27e2abc4a8a60490ccc2b79c2fc52d986957376dc47587c1dcdfba97324d84fde01bdfab99600000000000000000000000000000000000094357700000000010000000000000000480200000000000001020000000000000000000000000000000000000000000000000000000000000008000000000000007769746864726177e003000000000000010000000000000046c9885ca608aea8031fa10377eb80583467889a08ea49a87e90c26efcdee5d44cd39429d7863f8ca68507dfd20cc50e82b105a56f8b4cbf9c75657816b93ddd0eefddd5c9f88f1e1ea8f7af067796accb1c8bd727122a92997ebc449889fd0f9f3e87eb0b1d7d4531711291ebdb8673924886da9f9b9bb0502f9ef7b4161e22cb8c0cb0dad6dcd604ce4775d26d750a3a567117b5d12033edbf3aae24d1b0731d504b0fff9418670ca9c628ecd06c3665081fcbb4d044726d628f347c8c47040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000048020000000000000100000000000000dc81efe4d8026b9ec9de7566f884cdef8598dc561d353638788453e6671c37b8d5bbec72e8c5da4be5facf70729d4514dc3adaa5ef8f33abcabdfcc3472cab7d9c65bcbbd3623e4df0bba95d9613535729c338c2cd000bc64856d180746cb908000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000092d67f588500000046c9885ca608aea8031fa10377eb80583467889a08ea49a87e90c26efcdee5d44cd39429d7863f8ca68507dfd20cc50e82b105a56f8b4cbf9c75657816b93ddd0eefddd5c9f88f1e1ea8f7af067796accb1c8bd727122a92997ebc449889fd0f9f3e87eb0b1d7d4531711291ebdb8673924886da9f9b9bb0502f9ef7b4161e22cb8c0cb0dad6dcd604ce4775d26d750a3a567117b5d12033edbf3aae24d1b0731d504b0fff9418670ca9c628ecd06c3665081fcbb4d044726d628f347c8c4704000000000000000067b9cd554bf8edbd949be6472fac0d23ea504d98154e335883f0b9fe8fe2524d1e128ee8ba073d59e9e80c3d46b4430d2e05cb92d6523e012979be49cad0f99443f3f3a4c38ea2845fb11f54906dbd73be5badf19fe405a566c233eaf76dac17000000000000000067b9cd554bf8edbd949be6472fac0d23ea504d98154e335883f0b9fe8fe2524d1e128ee8ba073d59e9e80c3d46b4430d2e05cb92d6523e012979be49cad0f99443f3f3a4c38ea2845fb11f54906dbd73be5badf19fe405a566c233eaf76dac17000000000000000097949b2b47a207f3914f79fc10d18e89931c2875672e1e6811070162963926bc5627b74ec8e8e9945299d02c486d8de2").unwrap();



    let transfer_tx = dusk_core::transfer::Transaction::from_slice(&tx_vec)
        .map_err(|e| anyhow::anyhow!("Invalid transaction: {e:?}")).unwrap();
    // session.call(dusk_vm::transfer::TRANSFER_CONTRACT, "execute", )
    let r = dusk_vm::execute(&mut session, &transfer_tx, &dusk_vm::ExecutionConfig::default());
    println!("r={:?}", r);
    // let receipt = execute(&mut session, &transfer_tx, &config);



    Ok(())
}
// called `Result::unwrap()` on an `Err` value: SessionError("VM Error: No such base commit: 50e0c2334301c64e31a135c839a772a9240814613cd344cc80ff31f33acd46e9")

// r=Err(RuntimeError(wasm trap: out of bounds memory access))
