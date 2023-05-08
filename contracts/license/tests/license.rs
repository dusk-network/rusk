// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

#[allow(unused)]
#[path = "../src/license_types.rs"]
mod license_types;

#[path = "../src/license_circuits.rs"]
mod license_circuits;

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{
    dhke, JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{
    PublicKey, PublicSpendKey, SecretKey, SecretSpendKey, StealthAddress,
    ViewKey,
};
use dusk_plonk::prelude::*;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_schnorr::Signature;

use dusk_bytes::Serializable;
use dusk_poseidon::tree::PoseidonTree;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use ::license_circuits::LicenseCircuit;
use license_types::*;
use piecrust::{ModuleData, ModuleId, Session, VM};
use zk_citadel::license::{LicenseProverParameters, SessionCookie};

const DEPTH: usize = 17; // depth of the 4-ary Merkle tree
type Tree = PoseidonTree<DataLeaf, (), DEPTH>;

const LICENSE_CONTRACT_ID: ModuleId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xf8;
    ModuleId::from_bytes(bytes)
};

const POINT_LIMIT: u64 = 0x10000000;
const TEST_OWNER: [u8; 32] = [0; 32];
const USER_ATTRIBUTES: u64 = 545072475273;
const CHALLENGE: u64 = 20221126u64;

static LABEL: &[u8] = b"dusk-network";
const CAPACITY: usize = 17; // capacity required for the setup

fn create_test_license<R: RngCore + CryptoRng>(
    attr: &JubJubScalar,
    ssk_lp: &SecretSpendKey,
    psk_lp: &PublicSpendKey,
    sa_user: &StealthAddress,
    k_lic: &JubJubAffine,
    rng: &mut R,
) -> License {
    let request = create_test_request(psk_lp, sa_user, k_lic, rng);
    create_test_license_from_request(attr, ssk_lp, &request, rng)
}

fn create_test_request<R: RngCore + CryptoRng>(
    psk_lp: &PublicSpendKey,
    sa_user: &StealthAddress,
    k_lic: &JubJubAffine,
    rng: &mut R,
) -> Request {
    let nonce_1 = BlsScalar::random(rng);
    let nonce_2 = BlsScalar::random(rng);
    let nonce_3 = BlsScalar::random(rng);

    let lpk = JubJubAffine::from(*sa_user.pk_r().as_ref());
    let r = JubJubAffine::from(*sa_user.R());

    let r_dh = JubJubScalar::random(rng);
    let rsa = psk_lp.gen_stealth_address(&r_dh);
    let k_dh = dhke(&r_dh, psk_lp.A());

    let enc_1 =
        PoseidonCipher::encrypt(&[lpk.get_x(), lpk.get_y()], &k_dh, &nonce_1);

    let enc_2 =
        PoseidonCipher::encrypt(&[r.get_x(), r.get_y()], &k_dh, &nonce_2);

    let enc_3 = PoseidonCipher::encrypt(
        &[k_lic.get_x(), k_lic.get_y()],
        &k_dh,
        &nonce_3,
    );

    Request {
        rsa,
        enc_1,
        nonce_1,
        enc_2,
        nonce_2,
        enc_3,
        nonce_3,
    }
}

fn create_test_license_from_request<R: RngCore + CryptoRng>(
    attr: &JubJubScalar,
    ssk_lp: &SecretSpendKey,
    req: &Request,
    rng: &mut R,
) -> License {
    let k_dh = dhke(ssk_lp.a(), req.rsa.R());

    let dec_1 = req
        .enc_1
        .decrypt(&k_dh, &req.nonce_1)
        .expect("decryption should succeed");

    let dec_2 = req
        .enc_2
        .decrypt(&k_dh, &req.nonce_2)
        .expect("decryption should succeed");

    let dec_3 = req
        .enc_3
        .decrypt(&k_dh, &req.nonce_3)
        .expect("decryption should succeed");

    let lpk = JubJubAffine::from_raw_unchecked(dec_1[0], dec_1[1]);
    let r = JubJubAffine::from_raw_unchecked(dec_2[0], dec_2[1]);
    let k_lic = JubJubAffine::from_raw_unchecked(dec_3[0], dec_3[1]);

    let message =
        sponge::hash(&[lpk.get_x(), lpk.get_y(), BlsScalar::from(*attr)]);

    let sig_lic = Signature::new(&SecretKey::from(ssk_lp.a()), rng, message);
    let sig_lic_r = JubJubAffine::from(sig_lic.R());

    let nonce_1 = BlsScalar::random(rng);
    let nonce_2 = BlsScalar::random(rng);

    let enc_1 = PoseidonCipher::encrypt(
        &[BlsScalar::from(*sig_lic.u()), BlsScalar::from(*attr)],
        &k_lic,
        &nonce_1,
    );

    let enc_2 = PoseidonCipher::encrypt(
        &[sig_lic_r.get_x(), sig_lic_r.get_y()],
        &k_lic,
        &nonce_2,
    );

    let pos = BlsScalar::from(1u64);

    License {
        lsa: StealthAddress::from_raw_unchecked(
            JubJubExtended::from(r),
            PublicKey::from_raw_unchecked(JubJubExtended::from(lpk)),
        ),
        enc_1,
        nonce_1,
        enc_2,
        nonce_2,
        pos,
    }
}

fn create_test_license_params_cookie<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> (License, LicenseProverParameters, SessionCookie) {
    // User
    let ssk_user = SecretSpendKey::random(rng);
    let psk_user = ssk_user.public_spend_key();
    let sa_user = psk_user.gen_stealth_address(&JubJubScalar::random(rng));

    // License provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();

    let k_lic =
        JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));
    let req = create_test_request(&psk_lp, &sa_user, &k_lic, rng);

    let attr = JubJubScalar::from(USER_ATTRIBUTES);
    let lic = create_test_license_from_request(&attr, &ssk_lp, &req, rng);

    let c = JubJubScalar::from(CHALLENGE);
    let (lpp, sc) = create_test_parameters_and_cookie(
        &sa_user, &ssk_user, &lic, &psk_lp, &psk_lp, &k_lic, &c, rng,
    );

    (lic, lpp, sc)
}

fn create_test_parameters_and_cookie<R: RngCore + CryptoRng>(
    sa_user: &StealthAddress,
    ssk_user: &SecretSpendKey,
    lic: &License,
    psk_lp: &PublicSpendKey,
    psk_sp: &PublicSpendKey,
    k_lic: &JubJubAffine,
    c: &JubJubScalar,
    rng: &mut R,
) -> (LicenseProverParameters, SessionCookie) {
    let dec_1 = lic
        .enc_1
        .decrypt(k_lic, &lic.nonce_1)
        .expect("decryption should succeed");

    let dec_2 = lic
        .enc_2
        .decrypt(k_lic, &lic.nonce_2)
        .expect("decryption should succeed");

    let attr = JubJubScalar::from_bytes(&dec_1[1].to_bytes()).unwrap();
    let sig_lic = Signature::from_bytes(
        &[
            dec_1[0].to_bytes(),
            JubJubAffine::from_raw_unchecked(dec_2[0], dec_2[1]).to_bytes(),
        ]
        .concat()
        .try_into()
        .expect("slice with incorrect length"),
    )
    .unwrap();

    let lsk = ssk_user.sk_r(sa_user);
    let lpk_p = JubJubAffine::from(GENERATOR_NUMS_EXTENDED * lsk.as_ref());

    let s_0 = BlsScalar::random(rng);
    let s_1 = JubJubScalar::random(rng);
    let s_2 = JubJubScalar::random(rng);

    let pk_sp = JubJubAffine::from(*psk_sp.A());
    let r = BlsScalar::random(rng);

    let session_hash = sponge::hash(&[pk_sp.get_x(), pk_sp.get_y(), r]);

    let sig_session_hash = dusk_schnorr::Proof::new(&lsk, rng, session_hash);

    let session_id =
        sponge::hash(&[lpk_p.get_x(), lpk_p.get_y(), BlsScalar::from(*c)]);

    let pk_lp = JubJubAffine::from(*psk_lp.A());

    let com_0 = sponge::hash(&[pk_lp.get_x(), pk_lp.get_y(), s_0]);
    let com_1 = (GENERATOR_EXTENDED * attr) + (GENERATOR_NUMS_EXTENDED * s_1);
    let com_2 = (GENERATOR_EXTENDED * c) + (GENERATOR_NUMS_EXTENDED * s_2);

    let lpk = JubJubAffine::from(*sa_user.pk_r().as_ref());
    let license_hash = sponge::hash(&[lpk.get_x(), lpk.get_y()]);

    let mut tree = Tree::default();
    let pos_tree = tree.push(DataLeaf::new(license_hash, 0));

    for i in 1..1024 {
        let l = DataLeaf::from(i as u64);
        tree.push(l);
    }

    let merkle_proof =
        tree.branch(pos_tree).expect("Tree was read successfully");

    (
        LicenseProverParameters {
            lpk,
            lpk_p,
            sig_lic,

            com_0,
            com_1,
            com_2,

            session_hash,
            sig_session_hash,
            merkle_proof,
        },
        SessionCookie {
            pk_sp,
            r,
            session_id,
            pk_lp,
            attr,
            c: *c,
            s_0,
            s_1,
            s_2,
        },
    )
}

fn initialize() -> Session {
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");
    rusk_abi::register_host_queries(&mut vm);

    let bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/license.wasm"
    );

    let mut session = rusk_abi::session(&vm, None, 0)
        .expect("Instantiating a genesis session should succeed");

    session.set_point_limit(POINT_LIMIT);

    session
        .deploy(
            bytecode,
            ModuleData::builder(TEST_OWNER).module_id(LICENSE_CONTRACT_ID),
        )
        .expect("Deploying the license contract should succeed");

    session
}

#[test]
fn request_send_get() {
    let rng = &mut StdRng::seed_from_u64(0xcafe);
    let mut session = initialize();

    // user
    let ssk_user = SecretSpendKey::random(rng);
    let psk_user = ssk_user.public_spend_key();
    let sa_user = psk_user.gen_stealth_address(&JubJubScalar::random(rng));
    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();
    let view_key_lp = ViewKey::from(ssk_lp);

    let k_lic =
        JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::random(rng));
    let request = create_test_request(&psk_lp, &sa_user, &k_lic, rng);

    session
        .transact::<Request, ()>(
            LICENSE_CONTRACT_ID,
            "request_license",
            &request,
        )
        .expect("Requesting license should succeed");

    assert!(
        session
            .query::<ViewKey, Option<Request>>(
                LICENSE_CONTRACT_ID,
                "get_license_request",
                &view_key_lp,
            )
            .expect("Querying the license request should succeed")
            .is_some(),
        "First call to getting a license request should return some"
    );

    assert!(
        session
            .query::<StealthAddress, Option<Request>>(
                LICENSE_CONTRACT_ID,
                "get_license_request",
                &request.rsa,
            )
            .expect("Querying the license request should succeed")
            .is_none(),
        "Second call to getting a license request should return none"
    );
}

#[test]
fn license_issue_get() {
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

    session
        .transact::<License, ()>(LICENSE_CONTRACT_ID, "issue_license", &license)
        .expect("Issuing license should succeed");

    assert!(
        session
            .query::<StealthAddress, Option<License>>(
                LICENSE_CONTRACT_ID,
                "get_license",
                &license.lsa,
            )
            .expect("Querying the license should succeed")
            .is_some(),
        "First call to getting a license request should return some"
    );

    assert_eq!(
        session
            .query::<StealthAddress, Option<License>>(
                LICENSE_CONTRACT_ID,
                "get_license",
                &license.lsa,
            )
            .expect("Querying the license should succeed"),
        None,
        "First call to getting a license request should return none"
    );
}

#[test]
fn get_session_none() {
    const SESSION_ID: u64 = 7u64;
    let mut session = initialize();
    let session_id = SessionId::new(BlsScalar::from(SESSION_ID));

    let license_session = session
        .query::<SessionId, Option<license_types::Session>>(
            LICENSE_CONTRACT_ID,
            "get_session",
            &session_id,
        )
        .expect("Querying the session should succeed");

    assert_eq!(None::<license_types::Session>, license_session);
}

#[test]
fn use_license_get_session() {
    let rng = &mut StdRng::seed_from_u64(0xbeef);
    let mut session = initialize();

    let pp = PublicParameters::setup(1 << CAPACITY, rng).unwrap();

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

    let (prover, _verifier) = Compiler::compile::<LicenseCircuit>(&pp, LABEL)
        .expect("Compiling circuit should succeed");

    let (_lic, lpp, sc) = create_test_license_params_cookie(rng);

    let (proof, public_inputs) = prover
        .prove(rng, &LicenseCircuit::new(&lpp, &sc))
        .expect("Proving should succeed");

    let use_license_arg = UseLicenseArg {
        proof,
        public_inputs,
        license,
    };

    let session_id = session
        .transact::<UseLicenseArg, SessionId>(
            LICENSE_CONTRACT_ID,
            "use_license",
            &use_license_arg,
        )
        .expect("Use license should succeed");

    let license_session = session
        .query::<license_types::SessionId, license_types::Session>(
            LICENSE_CONTRACT_ID,
            "get_session",
            &session_id,
        )
        .expect("Get session should succeed");
    assert_eq!(license_session.session_id(), session_id);
}
