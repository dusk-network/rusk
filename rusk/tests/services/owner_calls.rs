// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use dusk_core::transfer::data::ContractCall;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use dusk_rusk_test::{
    BlsPublicKey, BlsSecretKey, Result, RuskVmConfig, TestContext,
};

use dusk_core::abi::ContractId;
use dusk_vm::{gen_contract_id, ContractData};
use tracing::info;

use crate::common::logger;

const POINT_LIMIT: u64 = 0x10000000;

const NON_BLS_OWNER: [u8; 32] = [1; 32];

const BOB_INIT_VALUE: u8 = 5;

const GUARDED_METHOD: &str = "owner_reset";

#[allow(dead_code)]
struct Fixture {
    pub tc: TestContext,
    pub contract_id: ContractId,
}

impl Fixture {
    async fn build(owner: impl AsRef<[u8]>) -> Self {
        let owner = owner.as_ref();
        let bob_bytecode = include_bytes!("../../../contracts/bin/bob.wasm");
        let mut rng = StdRng::from_entropy();
        let nonce = rng.gen();
        let contract_id = gen_contract_id(bob_bytecode, nonce, owner);
        let deploy_data = ContractData::builder()
            .owner(owner)
            .init_arg(&BOB_INIT_VALUE)
            .contract_id(contract_id);

        let state = include_str!("../config/contract_deployment.toml");
        let tc = TestContext::instantiate_with(
            state,
            RuskVmConfig::new(),
            |session| {
                session
                    .deploy(bob_bytecode, deploy_data, POINT_LIMIT)
                    .expect("Deploying the bob contract should succeed");
            },
        )
        .await
        .expect("Initializing should succeed");

        let original_root = tc.state_root();

        info!("Original Root: {:?}", hex::encode(original_root));

        Self { tc, contract_id }
    }

    pub fn assert_bob_contract_is_deployed(&self) {
        const BOB_ECHO_VALUE: u64 = 775;

        let result: Result<u64, _> =
            self.tc
                .rusk()
                .query(self.contract_id, "echo", &BOB_ECHO_VALUE);
        assert_eq!(result.expect("Echo call should succeed"), BOB_ECHO_VALUE);

        let result: u8 = self
            .tc
            .rusk()
            .query(self.contract_id, "value", &())
            .expect("Value call should succeed");
        assert_eq!(result, BOB_INIT_VALUE);
    }

    pub fn query_contract<R>(&mut self, method: impl AsRef<str>) -> Result<R>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.tc
            .rusk()
            .query(self.contract_id, method.as_ref(), &())
            .map_err(|e| {
                anyhow::anyhow!(
                    "Querying contract method {} failed: {e}",
                    method.as_ref()
                )
            })
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
fn make_owner_only_tx<'a, E: Into<Option<&'a str>>>(
    contract_id: ContractId,
    args: u8,
    fname: impl AsRef<str>,
    nonce: u64,
    owner_sk: &BlsSecretKey,
    tc: &TestContext,
    expected_error: E,
) {
    const GAS_LIMIT: u64 = 1_000_000_000;
    const GAS_PRICE: u64 = 1;
    let msg = OwnerMessage {
        contract_id,
        args,
        fname: fname.as_ref().to_string(),
        nonce,
    };
    let msg_bytes = rkyv::to_bytes::<_, 4096>(&msg)
        .expect("Message should serialize correctly")
        .to_vec();
    let sig = owner_sk.sign(&msg_bytes);

    let call = ContractCall::new(contract_id, fname.as_ref())
        .with_args(&(sig, msg))
        .expect("call to be created successfully");
    let tx = tc
        .wallet()
        .moonlight_execute(0, 0, 0, GAS_LIMIT, GAS_PRICE, Some(call))
        .expect("tx to be created successfully");
    let _ = tc.execute_transaction(tx, 1, expected_error);
}

#[tokio::test(flavor = "multi_thread")]
pub async fn bls_non_owner_guarded_call() -> Result<()> {
    logger();
    const VALUE: u8 = 244;
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let sk = BlsSecretKey::random(rng);

    let f = Fixture::build(NON_BLS_OWNER).await;
    f.assert_bob_contract_is_deployed();

    make_owner_only_tx(
        f.contract_id,
        VALUE,
        GUARDED_METHOD,
        0,
        &sk,
        &f.tc,
        "Panic: method restricted only to the owner",
    );
    Ok(())
}

/// owner is a BLS public key, method called is guarded
#[tokio::test(flavor = "multi_thread")]
pub async fn bls_owner_guarded_call() -> Result<()> {
    logger();
    const VALUE1: u8 = 244;
    const VALUE2: u8 = 233;
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let sk = BlsSecretKey::random(rng);
    let pk = BlsPublicKey::from(&sk);
    let owner = pk.to_bytes();

    let mut f = Fixture::build(&owner).await;
    f.assert_bob_contract_is_deployed();

    let nonce = f.query_contract::<u64>("nonce")?;
    make_owner_only_tx(
        f.contract_id,
        VALUE1,
        GUARDED_METHOD,
        nonce,
        &sk,
        &f.tc,
        None,
    );
    let value = f.query_contract::<u8>("value")?;
    assert_eq!(VALUE1, value);

    // make sure the next call will fail if we do not increase the nonce
    make_owner_only_tx(
        f.contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce,
        &sk,
        &f.tc,
        "Panic: method restricted only to the owner",
    );

    // next call should work if we increase the nonce
    make_owner_only_tx(
        f.contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce + 1,
        &sk,
        &f.tc,
        None,
    );
    let value = f.query_contract::<u8>("value")?;
    assert_eq!(VALUE2, value);

    // call should fail if method name is incorrect
    make_owner_only_tx(
        f.contract_id,
        VALUE2,
        "incorrect",
        nonce + 2,
        &sk,
        &f.tc,
        "Unknown",
    );

    // call should fail if contract id is incorrect
    let mut contract_id_bytes = f.contract_id.to_bytes();
    contract_id_bytes[0] ^= 0xff;
    let incorrect_contract_id = ContractId::from_bytes(contract_id_bytes);
    make_owner_only_tx(
        incorrect_contract_id,
        VALUE2,
        GUARDED_METHOD,
        nonce + 2,
        &sk,
        &f.tc,
        "Contract does not exist",
    );

    Ok(())
}
