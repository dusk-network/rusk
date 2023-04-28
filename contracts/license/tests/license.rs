// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

#[allow(unused)]
#[path = "../src/license_types.rs"]
mod license_types;

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{
    JubJubAffine, GENERATOR_EXTENDED,
};
use dusk_pki::SecretKey;
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use zk_citadel::gadget;
use zk_citadel::license::License;

use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use license_types::*;
use piecrust::{ModuleId, Session, VM};

const LICENSE_CONTRACT_ID: ModuleId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xf8;
    ModuleId::from_bytes(bytes)
};

const POINT_LIMIT: u64 = 0x10000000;

static LABEL: &[u8; 12] = b"dusk-network";
const CAPACITY: usize = 17; // capacity required for the setup

fn random_public_key<R: RngCore + CryptoRng>(rng: &mut R) -> JubJubAffine {
    let sk = SecretKey::random(rng);
    JubJubAffine::from(GENERATOR_EXTENDED * sk.as_ref())
}

fn random_license_user_pk<R: RngCore + CryptoRng>(rng: &mut R) -> (ContractLicense, UserPublicKey) {
    let user_pk = UserPublicKey {
        user_pk: random_public_key(rng),
    };
    let sp_pk = SPPublicKey {
        sp_pk: random_public_key(rng),
    };
    (ContractLicense {
        user_pk,
        sp_pk,
        sig_lic: Signature::default(),
    }, user_pk)
}

fn initialize() -> Session {
    let vm = VM::ephemeral().expect("Creating a VM should succeed");

    let bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/license.wasm"
    );

    let mut session = vm.genesis_session();

    session.set_point_limit(POINT_LIMIT);

    session
        .deploy_with_id(LICENSE_CONTRACT_ID, bytecode)
        .expect("Deploying the license contract should succeed");

    session
}

#[test]
fn request_set_get() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    let sp_pk = SPPublicKey {
        sp_pk: random_public_key(rng),
    };
    let license_request = LicenseRequest {
        sp_public_key: sp_pk,
    };

    session
        .transact::<LicenseRequest, ()>(
            LICENSE_CONTRACT_ID,
            "request_license",
            &license_request,
        )
        .expect("Requesting license should succeed");

    assert!(
        session
            .query::<SPPublicKey, Option<LicenseRequest>>(
                LICENSE_CONTRACT_ID,
                "get_license_request",
                &sp_pk,
            )
            .expect("Querying the license request should succeed")
            .is_some(),
        "First call to getting a license request should return some"
    );

    assert_eq!(
        session
            .query::<SPPublicKey, Option<LicenseRequest>>(
                LICENSE_CONTRACT_ID,
                "get_license_request",
                &sp_pk,
            )
            .expect("Querying the license request should succeed"),
        None,
        "Second call to getting a license request should return none"
    );
}

#[test]
fn license_issue_get() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    let (license, user_pk) = random_license_user_pk(rng);

    session
        .transact::<ContractLicense, ()>(
            LICENSE_CONTRACT_ID,
            "issue_license",
            &license,
        )
        .expect("Issuing license should succeed");

    assert!(
        session
            .query::<UserPublicKey, Option<ContractLicense>>(
                LICENSE_CONTRACT_ID,
                "get_license",
                &user_pk,
            )
            .expect("Querying the license should succeed")
            .is_some(),
        "First call to getting a license request should return some"
    );

    assert_eq!(
        session
            .query::<UserPublicKey, Option<ContractLicense>>(
                LICENSE_CONTRACT_ID,
                "get_license",
                &user_pk,
            )
            .expect("Querying the license should succeed"),
        None,
        "First call to getting a license request should return none"
    );
}

#[test]
fn get_session_none() {
    let mut session = initialize();

    let nullifier = LicenseNullifier {
        value: BlsScalar::from(7u64),
    };

    let license_session = session
        .query::<LicenseNullifier, Option<LicenseSession>>(
            LICENSE_CONTRACT_ID,
            "get_session",
            &nullifier,
        )
        .expect("Querying the session should succeed");

    assert_eq!(None::<LicenseSession>, license_session);
}

#[derive(Default, Debug)]
pub struct Citadel {
    license: License,
}

impl Citadel {
    pub fn new(license: License) -> Self {
        Self { license }
    }
}

impl Circuit for Citadel {
    fn circuit<C>(&self, composer: &mut C) -> Result<(), Error>
    where
        C: Composer,
    {
        gadget::nullify_license(composer, &self.license)?;
        Ok(())
    }
}

#[test]
fn use_license() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    let pp = PublicParameters::setup(1 << CAPACITY, rng).unwrap();
    let (l, _) = random_license_user_pk(rng); // todo, eliminate me
    let license = License::random(rng);

    let (prover, _verifier) = Compiler::compile::<Citadel>(&pp, LABEL)
        .expect("Compiling circuit should succeed");

    let (proof, public_inputs) = prover
        .prove(rng, &Citadel::new(license.clone()))
        .expect("Proving should succeed");

    let use_license_request = UseLicenseRequest {
        proof,
        public_inputs,
        license: l, // todo, replace with real license
    };

    session
        .transact::<UseLicenseRequest, ()>(
            LICENSE_CONTRACT_ID,
            "use_license",
            &use_license_request,
        )
        .expect("Use license should succeed");
}
