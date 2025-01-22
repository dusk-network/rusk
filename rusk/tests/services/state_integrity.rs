// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod base_info;
mod hash;
mod page_tree;
mod tree_pos;
mod utils;

use std::collections::{BTreeSet, HashMap};
use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::{fs, io};

use bytecheck::CheckBytes;
use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, ContractData, Session, VM};
use rkyv::{Archive, Deserialize, Serialize};
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
use crate::services::state_integrity::page_tree::PageTree;
use crate::services::state_integrity::tree_pos::TreePos;
use crate::services::state_integrity::utils::{
    calculate_root, calculate_root_pos_32, contract_id_from_hex,
    contract_prefix, find_commit_level, find_current_levels, find_element,
    find_file_path_at_level, position_from_contract, scan_commits, EDGE_DIR,
    ELEMENT_FILE, LEAF_DIR, MAIN_DIR,
};

const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const POINT_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const BOB_INIT_VALUE: u8 = 5;

const METHOD: &str = "reset";

const CHAIN_ID: u8 = 0xFA;

const STATE_DIR: &str = "/Users/miloszm/.dusk/rusk/state";

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

    pub fn create_session(&mut self, vm: &VM, commit: [u8; 32]) -> Session {
        vm.session(commit, CHAIN_ID, 0)
            .expect("Session creation should succeed")
    }

    pub fn create_vm(&mut self) -> VM {
        VM::new(self.path.as_path()).expect("VM creation should succeed")
    }
}

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractIndexElement {
    pub tree: PageTree,
    pub len: usize,
    pub page_indices: BTreeSet<usize>,
    pub hash: Option<[u8; 32]>,
    pub int_pos: Option<u64>,
}

fn load_tree_pos(
    path: impl AsRef<Path>,
    commit_id: &[u8; 32],
) -> Result<TreePos> {
    let file_path = path
        .as_ref()
        .join("main")
        .join(hex::encode(commit_id).as_str())
        .join("tree_pos_opt");
    let f = OpenOptions::new().read(true).open(file_path)?;
    let mut buf_f = BufReader::new(f);
    Ok(TreePos::unmarshall(&mut buf_f)?)
}

fn scan_elements(
    main_dir: impl AsRef<Path>,
    commit_id: &[u8; 32],
    level: u64,
    levels: &[u64],
) -> Result<Vec<([u8; 32], ContractId, u64, PathBuf)>> {
    let mut output = Vec::new();
    let leaf_dir = main_dir.as_ref().join(LEAF_DIR);
    for entry in fs::read_dir(&leaf_dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename == EDGE_DIR {
            continue;
        }
        if !entry.path().is_dir() {
            continue;
        }
        let contract_id_hex = filename;
        let contract_id = contract_id_from_hex(&contract_id_hex);
        let contract_leaf_path = leaf_dir.join(&contract_id_hex);
        let maybe_element_path =
            find_element(Some(*commit_id), &contract_leaf_path, &main_dir);
        let element_path = match maybe_element_path {
            None => find_file_path_at_level(
                &leaf_dir,
                level,
                &contract_id_hex,
                ELEMENT_FILE,
                levels,
            ),
            Some(p) => p,
        };
        println!("found element: {:?}", element_path);
        if element_path.is_file() {
            let element_bytes = fs::read(&element_path)?;
            let element: ContractIndexElement =
                rkyv::from_bytes(&element_bytes).map_err(|err| {
                    tracing::trace!(
                        "deserializing element file failed {}",
                        err
                    );
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Invalid element file \"{element_path:?}\": {err}"
                        ),
                    )
                })?;
            output.push((
                element.hash.unwrap_or([0; 32]),
                contract_id,
                element.int_pos.unwrap_or(0),
                element_path,
            ))
        }
    }
    Ok(output)
}

// commit A touches contract C resulting in C's state hash H1
// commit A is written (persisted but not finalized)
// commit B touches contract C resulting in C's state hash H2
// commit B is written (persisted but not finalized)
// commit A is finalized
// we expect effective hash of C to be H1, and not H2
fn perform_ops_leading_to_root_discrepancy(f: &mut Fixture) -> Result<(), Error> {
    f.assert_bob_contract_is_deployed();
    let vm = f.create_vm();
    let commit_id: [u8; 32] = f.rusk.state_root();

    let mut session_a = f.create_session(&vm, commit_id.clone());
    session_a
        .call::<u8, ()>(f.contract_id, METHOD, &0, u64::MAX)
        .map_err(Error::Vm)?;

    let commit_a = session_a.commit()?;

    let mut session_b = f.create_session(&vm, commit_a.clone());
    session_b
        .call::<u8, ()>(f.contract_id, METHOD, &1, u64::MAX)
        .map_err(Error::Vm)?;

    let commit_b = session_b.commit()?;

    vm.finalize_commit(commit_a.clone())?;

    println!("commit_b={}", hex::encode(commit_b));
    assert_eq!(hex::encode(get_state_root_of_commit(STATE_DIR, &commit_b)?), hex::encode(commit_b));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn verify_commits() -> Result<(), Error> {
    logger();
    let mut f = Fixture::build(NON_BLS_OWNER);

    perform_ops_leading_to_root_discrepancy(&mut f)?;

    verify_commits_roots()
    // Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn verify_state_roots() -> Result<(), Error> {
    verify_commits_roots()
}

fn verify_commits_roots() -> Result<(), Error> {
    let main_dir = PathBuf::from(STATE_DIR).join(MAIN_DIR);
    let commits = scan_commits(&main_dir)?;
    for commit in commits.iter() {
        verify_state_root_of_commit(STATE_DIR, commit)?;
    }
    Ok(())
}

fn verify_state_root_of_commit(
    state_dir: impl AsRef<Path>,
    commit_id: &[u8; 32],
) -> Result<(), Error> {
    println!();
    println!("tree_pos for commit {}", hex::encode(commit_id));
    let tree_pos = load_tree_pos(state_dir.as_ref(), commit_id)?;
    let mut tree_pos_map: HashMap<u64, [u8; 32]> = HashMap::new();
    for (k, (h, c)) in tree_pos.iter() {
        println!(
            "{} {} {}",
            *k,
            hex::encode(h),
            hex::encode((*c).to_le_bytes())
        );
        tree_pos_map.insert(*k as u64, h.clone());
    }

    let main_dir = state_dir.as_ref().join(MAIN_DIR);
    let level = find_commit_level(&main_dir, commit_id)?;
    let levels = find_current_levels(&main_dir)?;
    println!("level={} levels={:?}", level, levels);
    let mut elems = scan_elements(&main_dir, commit_id, level, &levels)?;
    elems.sort_by(|(_, _, pos1, _), (_, _, pos2, _)| pos1.cmp(pos2));
    println!();
    println!("elems:");
    for (hash, contract_id, int_pos, path_buf) in elems.iter() {
        let contract_pos_hex =
            hex::encode(position_from_contract(contract_id).to_le_bytes());
        println!();
        if Some(hash) != tree_pos_map.get(int_pos) {
            print!("* ");
        }
        print!(
            "{} {} ({}) int_pos={} path_buf={:?}",
            hex::encode(hash),
            contract_prefix(contract_id),
            contract_pos_hex,
            *int_pos,
            path_buf,
        );
    }

    let root_from_elements = calculate_root(
        elems.iter().map(|(hash, _, int_pos, _)| (hash, int_pos)),
    );
    println!();
    println!("root_from_elements={}", hex::encode(root_from_elements));
    let root_from_tree_pos_file =
        calculate_root_pos_32(tree_pos.iter().map(|(k, (h, _c))| (h, k)));
    println!();
    println!(
        "root_from_tree_pos_file={}",
        hex::encode(root_from_tree_pos_file)
    );

    println!();
    assert_eq!(
        hex::encode(root_from_elements),
        hex::encode(root_from_tree_pos_file)
    );

    Ok(())
}

fn get_state_root_of_commit(
    state_dir: impl AsRef<Path>,
    commit_id: &[u8; 32],
) -> Result<[u8; 32], Error> {
    let main_dir = state_dir.as_ref().join(MAIN_DIR);
    let level = find_commit_level(&main_dir, commit_id)?;
    println!("level={}", level);
    let levels = find_current_levels(&main_dir)?;
    println!("levels={:?}", levels);
    let elems = scan_elements(&main_dir, commit_id, level, &levels)?;
    let root_from_elements = calculate_root(
        elems.iter().map(|(hash, _, int_pos, _)| (hash, int_pos)),
    );
    Ok(root_from_elements)
}

// todo: this is really a utility, move it somewhere else or remove
#[tokio::test(flavor = "multi_thread")]
pub async fn show_el() -> Result<(), Error> {
    let levels = vec![2,3,4,5];
    for l in levels {
        println!("=== {} ===================", l);
        let element_path = PathBuf::from(STATE_DIR)
            .join(MAIN_DIR)
            .join(LEAF_DIR)
            .join("edge")
            .join(format!("{}", l))
            .join(
                "9b39e27695327ed5b9d95e11ddf817519d7c8dff529e205060d6854ab35d4f38",
            )
            .join(ELEMENT_FILE);
        if element_path.is_file() {
            let element_bytes = fs::read(&element_path)?;
            let element: ContractIndexElement = rkyv::from_bytes(&element_bytes)
                .map_err(|err| {
                    tracing::trace!("deserializing element file failed {}", err);
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid element file \"{element_path:?}\": {err}"),
                    )
                })?;
            println!(
                "{:?} - hash of element file: {:?}",
                element.hash.as_ref().map(|a| hex::encode(a)),
                element_path
            );
            println!("page indices={:?}", element.page_indices);
        } else {
            println!("{:?} - not a file", element_path);
        }
    }
    Ok(())
}
