// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, GENERATOR_EXTENDED};
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use phoenix_core::{
    PublicKey as PublicSpendKey, SecretKey as SecretSpendKey, StealthAddress,
};
use std::ops::Range;
use std::sync::mpsc;

use poseidon_merkle::Opening;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rkyv::{check_archived_root, Deserialize, Infallible};

#[path = "../src/license_types.rs"]
mod license_types;
use license_types::*;

use license_circuits::LicenseCircuit;

use rusk_abi::{ContractData, ContractId, Session};
use rusk_profile::get_common_reference_string;
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
        "../../../target/wasm32-unknown-unknown/release/license_contract.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(&vm);

    session
        .deploy(
            bytecode,
            ContractData::builder(TEST_OWNER).contract_id(LICENSE_CONTRACT_ID),
            POINT_LIMIT,
        )
        .expect("Deploying the license contract should succeed");

    session
}

/// Deserializes license, panics if deserialization fails.
fn deserialise_license(v: &Vec<u8>) -> License {
    let response_data = check_archived_root::<License>(v.as_slice())
        .expect("License should deserialize correctly");
    let license: License = response_data
        .deserialize(&mut Infallible)
        .expect("Infallible");
    license
}

/// Finds owned license in a collection of licenses.
/// It searches in a reverse order to return a newest license.
fn find_owned_license(
    ssk_user: SecretSpendKey,
    licenses: &Vec<(u64, Vec<u8>)>,
) -> Option<(u64, License)> {
    for (pos, license) in licenses.iter().rev() {
        let license = deserialise_license(&license);
        if ssk_user.view_key().owns(&license.lsa) {
            return Some((pos.clone(), license));
        }
    }
    None
}

/// Creates the Citadel request object
/// This function should be moved to CitadelUtils
fn create_request<R: RngCore + CryptoRng>(
    ssk_user: &SecretSpendKey,
    psk_lp: &PublicSpendKey,
    rng: &mut R,
) -> Request {
    let psk = ssk_user.public_spend_key();
    let lsa = psk.gen_stealth_address(&JubJubScalar::random(rng));
    let lsk = ssk_user.sk_r(&lsa);
    let k_lic = JubJubAffine::from(
        GENERATOR_EXTENDED * sponge::truncated::hash(&[(*lsk.as_ref()).into()]),
    );
    Request::new(psk_lp, &lsa, &k_lic, rng)
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

    let license =
        create_test_license(&attr, &ssk_lp, &psk_lp, &sa_user, &k_lic, rng);
    let license_blob = rkyv::to_bytes::<_, 4096>(&license)
        .expect("Request should serialize correctly")
        .to_vec();

    let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
    let license_hash = sponge::hash(&[lpk.get_u(), lpk.get_v()]);

    session
        .call::<(Vec<u8>, BlsScalar), ()>(
            LICENSE_CONTRACT_ID,
            "issue_license",
            &(license_blob, license_hash),
            POINT_LIMIT,
        )
        .expect("Issuing license should succeed");

    let bh_range = 0..10000u64;
    let (feeder, receiver) = mpsc::channel();
    session
        .feeder_call::<Range<u64>, ()>(
            LICENSE_CONTRACT_ID,
            "get_licenses",
            &bh_range,
            feeder,
        )
        .expect("Querying of the licenses should succeed")
        .data;

    let pos_license_pairs: Vec<(u64, Vec<u8>)> = receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return licenses"))
        .collect();

    assert!(
        !pos_license_pairs.is_empty(),
        "Call to getting a license request should return some licenses"
    );

    let owned_license = find_owned_license(ssk_user, &pos_license_pairs);
    assert!(
        owned_license.is_some(),
        "Some license should be owned by the user"
    );
    let (pos, _) = owned_license.unwrap();

    let _merkle_opening = session
        .call::<u64, Opening<(), DEPTH, ARITY>>(
            LICENSE_CONTRACT_ID,
            "get_merkle_opening",
            &pos,
            POINT_LIMIT,
        )
        .expect("Querying the merkle opening should succeed")
        .data;
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
    for _ in 0..NUM_LICENSES {
        let k_lic =
            JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));
        let license =
            create_test_license(&attr, &ssk_lp, &psk_lp, &sa_user, &k_lic, rng);
        let license_blob = rkyv::to_bytes::<_, 4096>(&license)
            .expect("Request should serialize correctly")
            .to_vec();

        let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
        let license_hash = sponge::hash(&[lpk.get_u(), lpk.get_v()]);
        session
            .call::<(Vec<u8>, BlsScalar), ()>(
                LICENSE_CONTRACT_ID,
                "issue_license",
                &(license_blob, license_hash),
                POINT_LIMIT,
            )
            .expect("Issuing license should succeed");
    }

    let (feeder, receiver) = mpsc::channel();
    let bh_range = 0..NUM_LICENSES as u64;
    session
        .feeder_call::<Range<u64>, ()>(
            LICENSE_CONTRACT_ID,
            "get_licenses",
            &bh_range,
            feeder,
        )
        .expect("Querying of the licenses should succeed")
        .data;

    let pos_license_pairs: Vec<(u64, Vec<u8>)> = receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return licenses"))
        .collect();

    assert_eq!(
        pos_license_pairs.len(),
        NUM_LICENSES,
        "Call to getting license requests should return licenses"
    );

    let owned_license = find_owned_license(ssk_user, &pos_license_pairs);
    assert!(
        owned_license.is_some(),
        "Some license should be owned by the user"
    );
    let (pos, _) = owned_license.unwrap();

    let _merkle_opening = session
        .call::<u64, Opening<(), DEPTH, ARITY>>(
            LICENSE_CONTRACT_ID,
            "get_merkle_opening",
            &pos,
            POINT_LIMIT,
        )
        .expect("Querying the merkle opening should succeed")
        .data;
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
            POINT_LIMIT,
        )
        .expect("Querying the session should succeed")
        .data;

    assert_eq!(None::<LicenseSession>, license_session);
}

#[test]
fn use_license_get_session() {
    let mut session = initialize();

    // NOTE: it is important that the seed is the same as in the recovery
    // PUB_PARAMS initialization code
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    let crs = get_common_reference_string().expect("getting CRS file works");
    let pp = unsafe { PublicParameters::from_slice_unchecked(crs.as_slice()) };

    let (prover, verifier) = Compiler::compile::<LicenseCircuit>(&pp, LABEL)
        .expect("Compiling circuit should succeed");

    // user
    let ssk_user = SecretSpendKey::random(rng);

    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();

    let request = create_request(&ssk_user, &psk_lp, rng);
    let attr = JubJubScalar::from(USER_ATTRIBUTES);
    let license = License::new(&attr, &ssk_lp, &request, rng);

    let license_blob = rkyv::to_bytes::<_, 4096>(&license)
        .expect("Request should serialize correctly")
        .to_vec();

    let lpk = JubJubAffine::from(license.lsa.pk_r().as_ref());
    let license_hash = sponge::hash(&[lpk.get_u(), lpk.get_v()]);

    session
        .call::<(Vec<u8>, BlsScalar), ()>(
            LICENSE_CONTRACT_ID,
            "issue_license",
            &(license_blob, license_hash),
            POINT_LIMIT,
        )
        .expect("Issuing license should succeed");

    let (feeder, receiver) = mpsc::channel();
    let bh_range = 0..10000u64;
    session
        .feeder_call::<Range<u64>, ()>(
            LICENSE_CONTRACT_ID,
            "get_licenses",
            &bh_range,
            feeder,
        )
        .expect("Querying the license should succeed")
        .data;

    let pos_license_pairs: Vec<(u64, Vec<u8>)> = receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return licenses"))
        .collect();

    assert!(
        !pos_license_pairs.is_empty(),
        "Call to getting license requests should return licenses"
    );

    let owned_license = find_owned_license(ssk_user, &pos_license_pairs);
    assert!(
        owned_license.is_some(),
        "Some license should be owned by the user"
    );
    let (pos, owned_license) = owned_license.unwrap();

    let merkle_opening = session
        .call::<u64, Opening<(), DEPTH, ARITY>>(
            LICENSE_CONTRACT_ID,
            "get_merkle_opening",
            &pos,
            POINT_LIMIT,
        )
        .expect("Querying the merkle opening should succeed")
        .data;

    let (cpp, sc) = CitadelUtils::compute_citadel_parameters(
        rng,
        ssk_user,
        psk_lp,
        &owned_license,
        merkle_opening,
    );
    let circuit = LicenseCircuit::new(cpp, sc);

    let (proof, public_inputs) =
        prover.prove(rng, &circuit).expect("Proving should succeed");

    let session_id = LicenseSessionId {
        id: public_inputs[0],
    };

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");

    let use_license_arg = UseLicenseArg {
        proof,
        public_inputs,
    };

    session
        .call::<UseLicenseArg, ()>(
            LICENSE_CONTRACT_ID,
            "use_license",
            &use_license_arg,
            POINT_LIMIT,
        )
        .expect("Use license should succeed");

    assert!(
        session
            .call::<LicenseSessionId, Option<LicenseSession>>(
                LICENSE_CONTRACT_ID,
                "get_session",
                &session_id,
                POINT_LIMIT
            )
            .expect("Get session should succeed")
            .data
            .is_some(),
        "Call to get session should return a session"
    );
}

#[test]
fn test_request_license() {
    let mut session = initialize();
    session
        .call::<(), ()>(
            LICENSE_CONTRACT_ID,
            "request_license",
            &(),
            POINT_LIMIT,
        )
        .expect("Request license should succeed");
}
