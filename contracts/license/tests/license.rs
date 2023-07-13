// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

#[path = "../src/license_circuits.rs"]
mod license_circuits;

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, GENERATOR_EXTENDED};
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress};
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use std::ops::Range;

use poseidon_merkle::Opening;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

#[path = "../src/license_types.rs"]
mod license_types;
use license_types::*;

use ::license_circuits::LicenseCircuit;

use rusk_abi::{ContractData, ContractId, Session};
use zk_citadel::license::{License, Request};
use zk_citadel::utils::CitadelUtils;

const LICENSE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xf8;
    ContractId::from_bytes(bytes)
};

const POINT_LIMIT: u64 = 0x10000000;
const TEST_OWNER: [u8; 32] = [0; 32];
const USER_ATTRIBUTES: u64 = 545072475273;

static LABEL: &[u8] = b"dusk-network";
const CAPACITY: usize = 17; // capacity required for the setup
const DEPTH: usize = 17; // depth of the Merkle tree
const ARITY: usize = 4; // arity of the Merkle tree

fn create_test_license<R: RngCore + CryptoRng>(
    attr: &JubJubScalar,
    ssk_lp: &SecretSpendKey,
    psk_lp: &PublicSpendKey,
    sa_user: &StealthAddress,
    k_lic: &JubJubAffine,
    rng: &mut R,
) -> License {
    let request = Request::new(psk_lp, sa_user, k_lic, rng);
    License::new(attr, ssk_lp, &request, rng)
}

fn initialize() -> Session {
    let vm =
        rusk_abi::new_ephemeral_vm().expect("Creating a VM should succeed");

    let bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/license.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(&vm);

    session.set_point_limit(POINT_LIMIT);

    session
        .deploy(
            bytecode,
            ContractData::builder(TEST_OWNER).contract_id(LICENSE_CONTRACT_ID),
        )
        .expect("Deploying the license contract should succeed");

    session
}

#[test]
fn license_issue_get_merkle() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    // user
    let ssk_user = SecretSpendKey::random(rng);
    let psk_user = ssk_user.public_spend_key();
    let sa_user = psk_user.gen_stealth_address(&JubJubScalar::random(rng));

    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();
    let k_lic =
        JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));

    let attr = JubJubScalar::from(USER_ATTRIBUTES);

    let mut license =
        create_test_license(&attr, &ssk_lp, &psk_lp, &sa_user, &k_lic, rng);
    license.pos = 1u64;
    let license_blob = rkyv::to_bytes::<_, 4096>(&license)
        .expect("Request should serialize correctly")
        .to_vec();

    let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
    let license_hash = sponge::hash(&[lpk.get_x(), lpk.get_y()]);

    session
        .call::<(Vec<u8>, u64, BlsScalar), ()>(
            LICENSE_CONTRACT_ID,
            "issue_license",
            &(license_blob, license.pos, license_hash),
        )
        .expect("Issuing license should succeed");

    let bh_range = 0..1u64;

    let licenses = session
        .call::<Range<u64>, Vec<Vec<u8>>>(
            LICENSE_CONTRACT_ID,
            "get_licenses",
            &bh_range,
        )
        .expect("Querying the licenses should succeed");

    assert_eq!(
        licenses.len(),
        1,
        "Call to getting a license request should return some licenses"
    );

    let merkle_opening = session
        .call::<u64, Opening<(), DEPTH, ARITY>>(
            LICENSE_CONTRACT_ID,
            "get_merkle_opening",
            &license.pos,
        )
        .expect("Querying the merkle opening should succeed");

    const EXPECTED_POSITIONS: [usize; DEPTH] =
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

    assert_eq!(merkle_opening.positions(), &EXPECTED_POSITIONS);
}

#[test]
fn multiple_licenses_issue_get_merkle() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    // user
    let ssk_user = SecretSpendKey::random(rng);
    let psk_user = ssk_user.public_spend_key();
    let sa_user = psk_user.gen_stealth_address(&JubJubScalar::random(rng));

    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();

    let attr = JubJubScalar::from(USER_ATTRIBUTES);

    const NUM_LICENSES: usize = ARITY + 1;
    for pos in 0..NUM_LICENSES {
        let k_lic =
            JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));
        let mut license =
            create_test_license(&attr, &ssk_lp, &psk_lp, &sa_user, &k_lic, rng);
        let license_blob = rkyv::to_bytes::<_, 4096>(&license)
            .expect("Request should serialize correctly")
            .to_vec();

        let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
        let license_hash = sponge::hash(&[lpk.get_x(), lpk.get_y()]);
        license.pos = pos as u64 + 1;
        session
            .call::<(Vec<u8>, u64, BlsScalar), ()>(
                LICENSE_CONTRACT_ID,
                "issue_license",
                &(license_blob, license.pos, license_hash),
            )
            .expect("Issuing license should succeed");
    }

    let bh_range = 0..NUM_LICENSES as u64;

    let licenses = session
        .call::<Range<u64>, Vec<Vec<u8>>>(
            LICENSE_CONTRACT_ID,
            "get_licenses",
            &bh_range,
        )
        .expect("Querying the license should succeed");

    assert_eq!(
        licenses.len(),
        NUM_LICENSES,
        "Call to getting license requests should return licenses"
    );

    let merkle_opening = session
        .call::<u64, Opening<(), DEPTH, ARITY>>(
            LICENSE_CONTRACT_ID,
            "get_merkle_opening",
            &(NUM_LICENSES as u64),
        )
        .expect("Querying the merkle opening should succeed");

    assert!(merkle_opening.positions()[DEPTH - 1] > 0);
    assert!(merkle_opening.positions()[DEPTH - 2] > 0);
}

#[test]
fn session_not_found() {
    const SESSION_ID: u64 = 7u64;
    let mut session = initialize();
    let session_id = LicenseSessionId {
        id: BlsScalar::from(SESSION_ID),
    };

    let license_session = session
        .call::<LicenseSessionId, Option<LicenseSession>>(
            LICENSE_CONTRACT_ID,
            "get_session",
            &session_id,
        )
        .expect("Querying the session should succeed");

    assert_eq!(None::<LicenseSession>, license_session);
}

#[test]
fn use_license_get_session() {
    let mut session = initialize();

    // NOTE: it is important that the seed is the same as in the recovery
    // PUB_PARAMS initialization code
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let pp = PublicParameters::setup(1 << CAPACITY, rng).unwrap();

    let (prover, verifier) = Compiler::compile::<LicenseCircuit>(&pp, LABEL)
        .expect("Compiling circuit should succeed");

    // user
    let ssk_user = SecretSpendKey::random(rng);
    let psk_user = ssk_user.public_spend_key();
    let sa_user = psk_user.gen_stealth_address(&JubJubScalar::random(rng));

    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();
    let k_lic =
        JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));

    let attr = JubJubScalar::from(USER_ATTRIBUTES);

    let mut license =
        create_test_license(&attr, &ssk_lp, &psk_lp, &sa_user, &k_lic, rng);
    license.pos = 1u64;
    let license_blob = rkyv::to_bytes::<_, 4096>(&license)
        .expect("Request should serialize correctly")
        .to_vec();

    let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
    let license_hash = sponge::hash(&[lpk.get_x(), lpk.get_y()]);

    session
        .call::<(Vec<u8>, u64, BlsScalar), ()>(
            LICENSE_CONTRACT_ID,
            "issue_license",
            &(license_blob, license.pos, license_hash),
        )
        .expect("Issuing license should succeed");

    let (cpp, sc) =
        CitadelUtils::compute_citadel_parameters::<StdRng, DEPTH, ARITY>(
            rng, ssk_user, psk_user, ssk_lp, psk_lp,
        );
    let circuit = LicenseCircuit::new(&cpp, &sc);

    let (proof, public_inputs) =
        prover.prove(rng, &circuit).expect("Proving should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");

    let use_license_arg = UseLicenseArg {
        proof,
        public_inputs,
    };

    let session_id = session
        .call::<UseLicenseArg, LicenseSessionId>(
            LICENSE_CONTRACT_ID,
            "use_license",
            &use_license_arg,
        )
        .expect("Use license should succeed");

    assert!(
        session
            .call::<LicenseSessionId, Option<LicenseSession>>(
                LICENSE_CONTRACT_ID,
                "get_session",
                &session_id,
            )
            .expect("Get session should succeed")
            .is_some(),
        "Call to get session should return a session"
    );
}

#[test]
fn test_noop() {
    let mut session = initialize();
    session
        .call::<(), ()>(LICENSE_CONTRACT_ID, "noop", &())
        .expect("Noop should succeed");
}
