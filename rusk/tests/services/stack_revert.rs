// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::operations::StateTransitionData;
use dusk_core::signatures::bls;
use node_data::ledger::Transaction;
use rusk::node::{DriverStore, RuskVmConfig, FEATURE_ABI_PUBLIC_SENDER};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, ContractData, Session, VM};
use rusk::{Error, Result, Rusk, DUSK_CONSENSUS_KEY};
use rusk_recovery_tools::state;
use rusk_recovery_tools::state::restore_state;
use tempfile::{tempdir, TempDir};
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

const CHAIN_ID: u8 = 0x01;

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

    let mut vm_config = RuskVmConfig::new().with_block_gas_limit(10_000_000_000);
    vm_config.with_feature(FEATURE_ABI_PUBLIC_SENDER, 1);

    let rusk = Rusk::new(
        dir,
        CHAIN_ID,
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

#[allow(dead_code)]
struct Fixture {
    pub rusk: Rusk,
    pub wallet: Wallet<TestStore, TestStateClient>,
    pub tmpdir: TempDir,
}

const NEW_FN: &str = "chain_id";
const OLD_FN: &str = "value";

impl Fixture {
    async fn build(owner: impl AsRef<[u8]>) -> Self {
        let tmpdir: TempDir = tempfile::tempdir().expect("tempdir() to work");
        let state_dir = tmpdir.path().join("state");
        let data = include_bytes!("../assets/2710377_state.tar.gz");

        let unarchive = rusk_recovery_tools::state::tar::unarchive(
            &data[..],
            state_dir.as_path(),
        )
        .expect("unarchive should work");

        // let state_dir = Path::new("/Users/seppia/Downloads/2710477 2/state
        // 2");

        let rusk = initial_state(&state_dir, owner.as_ref())
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

        Self {
            rusk,
            wallet,
            tmpdir,
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_ab() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER).await;
    println!("root={}", hex::encode(f.rusk.state_root()));
    // let base = hex::decode(
    //     "c5870f263709d39e5d8098fb53cbe17856f7d7a0ff47ccd47f5cea3687566bdb",
    // // Block 2,710,377 )
    let base = hex::decode(
        "53de818894cf665f1131edda3c5579ccb8736fd05c993ecb5cd16677974b088b", // Block 2,710,376
    )
    .unwrap();
    let mut base_a: [u8; 32] = [0u8; 32];
    base_a.copy_from_slice(&base);
    let mut lock = f.rusk.tip.write();
    lock.current = base_a;
    drop(lock);
    // let root = f.rusk.state_root();

    let activate_tx=hex::decode("01420300000000000001a363952604c1d55b0ba644c38abf4f2980875f4f19cd0476622d2621df3b5059d7329df20715b81f4836218d7bcf5d000bda33f028a419e09d7e427c6d123728510080d2361ea5d745de670b423e652a49acc30b3f3548c401b069f809b44b87000000000000000000000000000000000000943577000000000100000000000000001500000000000000016fdfdc713a18fc6ca2ad20eb2b4a3305a935ef47d6a872d9a4df8bc9fd9d169e0e000000000000007374616b655f61637469766174657802000000000000011cc415d05b1cfbf2583bf2e8a0e39b2c768d263ef92d6a21a4787f76c6afa9240000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000083993b79ec93685aae5f6f261565e464e8cc776af7e1fe67decb525bb9302859ee0831cc07f9dc0948b15045ada06b0cff57b1bf999f4bdc1883d2cec11ae1016f64d542364b0f9358efef0d7794bcec4b1310fadaedfcf3ab59e534c98a3e0f5bda8aa2e60537abcb1c92d8494cc1e0b42fb64748cf622ca175633260df994209f8ba3d59dcd3fe22611f04ba98560751c4384588aeb9ed93e5cafe9f0e6a949fb7e3c2dcb47c0cd36716e711b015d21da1a2b24361cdfedc81144d104644050000000000000000d83f0c3b05000000f78258909c3e094b0b0fa1a241942f5968599d23778328331fc478ced1f70ee1a42815977a2b0f7a309ca9b4cad98403b738ac9db702d39adaa94b3d71faaaac084e348c9b9e315e73b41c77494de8c69ecd98f4ac319981bb1632d8a8ee8e060000000000000000f78258909c3e094b0b0fa1a241942f5968599d23778328331fc478ced1f70ee1a42815977a2b0f7a309ca9b4cad98403b738ac9db702d39adaa94b3d71faaaac084e348c9b9e315e73b41c77494de8c69ecd98f4ac319981bb1632d8a8ee8e060000000000000000010000000000000087e8057bf2f732000379595a17979162773c6683226f92f869c9220e074759d3718e929e2b0af413fa3d0fbe355ef91a").unwrap();
    let stake_tx = hex::decode("01a40400000000000001942fc94c5fed7c4925c27361a552b4290b8a1f1d0584671d19f4cb5ac4d13307c3d87b274bb403f2d153bc9f8dcacf3c010ccd7438df51154f5df27e2abc4a8a60490ccc2b79c2fc52d986957376dc47587c1dcdfba97324d84fde01bdfab99600000000000000000000000000000000000094357700000000010000000000000000480200000000000001020000000000000000000000000000000000000000000000000000000000000008000000000000007769746864726177e003000000000000010000000000000046c9885ca608aea8031fa10377eb80583467889a08ea49a87e90c26efcdee5d44cd39429d7863f8ca68507dfd20cc50e82b105a56f8b4cbf9c75657816b93ddd0eefddd5c9f88f1e1ea8f7af067796accb1c8bd727122a92997ebc449889fd0f9f3e87eb0b1d7d4531711291ebdb8673924886da9f9b9bb0502f9ef7b4161e22cb8c0cb0dad6dcd604ce4775d26d750a3a567117b5d12033edbf3aae24d1b0731d504b0fff9418670ca9c628ecd06c3665081fcbb4d044726d628f347c8c47040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000048020000000000000100000000000000dc81efe4d8026b9ec9de7566f884cdef8598dc561d353638788453e6671c37b8d5bbec72e8c5da4be5facf70729d4514dc3adaa5ef8f33abcabdfcc3472cab7d9c65bcbbd3623e4df0bba95d9613535729c338c2cd000bc64856d180746cb908000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000092d67f588500000046c9885ca608aea8031fa10377eb80583467889a08ea49a87e90c26efcdee5d44cd39429d7863f8ca68507dfd20cc50e82b105a56f8b4cbf9c75657816b93ddd0eefddd5c9f88f1e1ea8f7af067796accb1c8bd727122a92997ebc449889fd0f9f3e87eb0b1d7d4531711291ebdb8673924886da9f9b9bb0502f9ef7b4161e22cb8c0cb0dad6dcd604ce4775d26d750a3a567117b5d12033edbf3aae24d1b0731d504b0fff9418670ca9c628ecd06c3665081fcbb4d044726d628f347c8c4704000000000000000067b9cd554bf8edbd949be6472fac0d23ea504d98154e335883f0b9fe8fe2524d1e128ee8ba073d59e9e80c3d46b4430d2e05cb92d6523e012979be49cad0f99443f3f3a4c38ea2845fb11f54906dbd73be5badf19fe405a566c233eaf76dac17000000000000000067b9cd554bf8edbd949be6472fac0d23ea504d98154e335883f0b9fe8fe2524d1e128ee8ba073d59e9e80c3d46b4430d2e05cb92d6523e012979be49cad0f99443f3f3a4c38ea2845fb11f54906dbd73be5badf19fe405a566c233eaf76dac17000000000000000097949b2b47a207f3914f79fc10d18e89931c2875672e1e6811070162963926bc5627b74ec8e8e9945299d02c486d8de2").unwrap();
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(64);
    let mut voters = vec![];
    for i in 0..10 {
        let sk = bls::SecretKey::random(&mut rng);
        let pk = bls::PublicKey::from(&sk);
        voters.push((node_data::bls::PublicKey::new(pk), i))
    }
    let data = StateTransitionData {
        round: 2710377,
        generator: node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY),
        slashes: vec![],
        cert_voters: voters,
        max_txs_bytes: 5000,
        prev_state_root: base_a,
    };

    let activate_tx = dusk_core::transfer::Transaction::from_slice(&activate_tx)
        .map_err(|e| anyhow::anyhow!("Invalid transaction: {e:?}"))
        .unwrap();
    let stake_tx = dusk_core::transfer::Transaction::from_slice(&stake_tx)
        .map_err(|e| anyhow::anyhow!("Invalid transaction: {e:?}"))
        .unwrap();
    let txs = vec![Transaction::from(activate_tx), Transaction::from(stake_tx)];
    let r = f.rusk.create_state_transition(&data, txs.into_iter());
    println!("r={:?}", r);
    // let receipt = execute(&mut session, &transfer_tx, &config);

    Ok(())
}
