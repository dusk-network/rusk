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
use dusk_jubjub::{JubJubAffine, GENERATOR_EXTENDED};
use dusk_pki::SecretKey;
use dusk_schnorr::Signature;
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

fn random_public_key<R: RngCore + CryptoRng>(rng: &mut R) -> JubJubAffine {
    let sk = SecretKey::random(rng);
    JubJubAffine::from(GENERATOR_EXTENDED * sk.as_ref())
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

    let _license_request = session
        .query::<SPPublicKey, LicenseRequest>(
            LICENSE_CONTRACT_ID,
            "get_license_request",
            &sp_pk,
        )
        .expect("Querying the license request should succeed");
}

#[test]
fn license_issue_get() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    let user_pk = UserPublicKey {
        user_pk: random_public_key(rng),
    };
    let sp_pk = SPPublicKey {
        sp_pk: random_public_key(rng),
    };
    let license = License {
        user_pk,
        sp_pk,
        sig_lic: Signature::default(),
    };

    session
        .transact::<License, ()>(LICENSE_CONTRACT_ID, "issue_license", &license)
        .expect("Issuing license should succeed");

    let _license = session
        .query::<UserPublicKey, License>(
            LICENSE_CONTRACT_ID,
            "get_license",
            &user_pk,
        )
        .expect("Querying the license should succeed");
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

#[test]
fn use_license() {
    let mut _session = initialize();
}
