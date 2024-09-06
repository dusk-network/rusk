// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use execution_core::{
    signatures::bls::{
        PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
        Signature as BlsSignature,
    },
    ContractId,
};
use rusk::gen_id::gen_contract_id;
use rusk::{Error, Result, Rusk};
use rusk_abi::{CallReceipt, ContractData, Session};
use rusk_recovery_tools::state;
use tempfile::tempdir;
use test_wallet::{self as wallet, Wallet};
use tokio::sync::broadcast;
use tracing::info;

use crate::common::logger;
use crate::common::wallet::{TestStateClient, TestStore};

const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;
const POINT_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const BOB_INIT_VALUE: u8 = 5;

const GUARDED_METHOD: &str = "owner_reset";

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
        None,
        None,
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
    pub session: Option<Session>,
}

impl Fixture {
    fn build(owner: impl AsRef<[u8]>) -> Self {
        let tmp =
            tempdir().expect("Should be able to create temporary directory");
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

        let path = tmp.into_path();
        Self {
            rusk,
            wallet,
            bob_bytecode,
            contract_id,
            path,
            session: None,
        }
    }

    pub fn assert_bob_contract_is_deployed(&self) {
        const BOB_ECHO_VALUE: u64 = 775;
        let commit = self.rusk.state_root();
        let vm = rusk_abi::new_vm(self.path.as_path())
            .expect("VM creation should succeed");
        let mut session = rusk_abi::new_session(&vm, commit, CHAIN_ID, 0)
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

    pub fn set_session(&mut self) {
        let commit = self.rusk.state_root();
        let vm = rusk_abi::new_vm(self.path.as_path())
            .expect("VM creation should succeed");
        self.session = Some(
            rusk_abi::new_session(&vm, commit, CHAIN_ID, 0)
                .expect("Session creation should succeed"),
        );
    }

    pub fn query_contract<R>(
        &mut self,
        method: impl AsRef<str>,
    ) -> Result<CallReceipt<R>, Error>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        assert!(self.session.is_some());
        self.session
            .as_mut()
            .unwrap()
            .call::<_, R>(self.contract_id, method.as_ref(), &(), u64::MAX)
            .map_err(Error::Vm)
    }
}

// this struct needs to be rkyv-serialization compatible between
// the contract caller and the contract, i.e., it doesn't need to be
// identical but it needs to rkyv-serialize to an identical slice of bytes
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct OwnerMessage {
    pub contract_id: ContractId,
    pub args: u8,
    pub fname: String,
    pub nonce: u64,
}

// performs a call to a method which may verify that it is called by the owner
fn make_owner_only_call<R>(
    contract_id: ContractId,
    args: u8,
    fname: impl AsRef<str>,
    nonce: u64,
    owner_sk: &BlsSecretKey,
    session: &mut Session,
) -> Result<CallReceipt<R>, Error>
where
    R: Archive,
    R::Archived:
        Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
{
    let msg = OwnerMessage {
        contract_id,
        args,
        fname: fname.as_ref().to_string(),
        nonce,
    };
    let msg_bytes = rkyv::to_bytes::<_, 4096>(&msg)
        .expect("Message should serialize correctly")
        .to_vec();
    let sig: BlsSignature = owner_sk.sign(&msg_bytes);
    let args = (sig, msg.clone());
    session
        .call::<(BlsSignature, OwnerMessage), R>(
            contract_id,
            fname.as_ref(),
            &args,
            u64::MAX,
        )
        .map_err(Error::Vm)
}

#[tokio::test(flavor = "multi_thread")]
pub async fn non_bls_owner_guarded_call() -> Result<(), Error> {
    logger();
    const VALUE: u8 = 244;
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let sk = BlsSecretKey::random(rng);
    let mut f = Fixture::build(NON_BLS_OWNER);
    f.assert_bob_contract_is_deployed();
    f.set_session();

    let r = make_owner_only_call::<()>(
        f.contract_id,
        VALUE,
        GUARDED_METHOD,
        0,
        &sk,
        f.session.as_mut().unwrap(),
    );
    assert!(r.is_err());
    Ok(())
}

/// owner is a BLS public key, method called is guarded
#[tokio::test(flavor = "multi_thread")]
pub async fn bls_owner_guarded_call() -> Result<(), Error> {
    logger();
    const VALUE1: u8 = 244;
    const VALUE2: u8 = 233;
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let sk = BlsSecretKey::random(rng);
    let pk = BlsPublicKey::from(&sk);
    let owner = pk.to_bytes();

    let mut f = Fixture::build(&owner);
    f.assert_bob_contract_is_deployed();
    f.set_session();

    let nonce = f.query_contract::<u64>("nonce")?.data;

    make_owner_only_call::<()>(
        f.contract_id,
        VALUE1,
        GUARDED_METHOD,
        nonce,
        &sk,
        f.session.as_mut().unwrap(),
    )?;
    let value = f.query_contract::<u8>("value")?;
    assert_eq!(VALUE1, value.data);

    // make sure the next call will fail if we do not increase the nonce
    let r = make_owner_only_call::<()>(
        f.contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce,
        &sk,
        f.session.as_mut().unwrap(),
    );
    assert!(r.is_err());

    // next call should work if we increase the nonce
    make_owner_only_call::<()>(
        f.contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce + 1,
        &sk,
        f.session.as_mut().unwrap(),
    )?;
    let value = f.query_contract::<u8>("value")?;
    assert_eq!(VALUE2, value.data);

    // call should fail if method name is incorrect
    let r = make_owner_only_call::<()>(
        f.contract_id,
        VALUE2,
        "incorrect",
        nonce + 2,
        &sk,
        f.session.as_mut().unwrap(),
    );
    assert!(r.is_err());

    // call should fail if contract id is incorrect
    let mut contract_id_bytes = f.contract_id.to_bytes();
    contract_id_bytes[0] ^= 0xff;
    let incorrect_contract_id = ContractId::from_bytes(contract_id_bytes);
    let r = make_owner_only_call::<()>(
        incorrect_contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce + 2,
        &sk,
        f.session.as_mut().unwrap(),
    );
    assert!(r.is_err());

    Ok(())
}
