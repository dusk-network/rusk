// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use dusk_poseidon::{Domain, Hash};
use execution_core::{
    license::{LicenseOpening, LicenseTree, LicenseTreeItem},
    transfer::phoenix::{PublicKey, SecretKey},
    JubJubAffine, GENERATOR_EXTENDED,
};
use ff::Field;
use license_circuits::{Error, LicenseCircuit, DEPTH};
use zk_citadel::license::{
    CitadelProverParameters, License, Request, SessionCookie,
};

use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

fn compute_citadel_parameters(
    rng: &mut StdRng,
    sk: &SecretKey,
    pk_lp: &PublicKey,
    lic: &License,
    merkle_proof: LicenseOpening,
) -> (CitadelProverParameters<DEPTH>, SessionCookie) {
    const CHALLENGE: u64 = 20221126u64;
    let c = JubJubScalar::from(CHALLENGE);
    let (cpp, sc) = CitadelProverParameters::compute_parameters(
        sk,
        lic,
        pk_lp,
        pk_lp,
        &c,
        rng,
        merkle_proof,
    )
    .expect("Parameters computed correctly.");
    (cpp, sc)
}

fn compute_random_license<R: RngCore + CryptoRng, const DEPTH: usize>(
    rng: &mut R,
    sk: &SecretKey,
    pk: PublicKey,
    sk_lp: &SecretKey,
    pk_lp: PublicKey,
) -> (License, LicenseOpening) {
    const ATTRIBUTE_DATA: u64 = 112233445566778899u64;
    // First, the user computes these values and requests a License
    let lsa = pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let lsk = sk.gen_note_sk(&lsa);
    let k_lic = JubJubAffine::from(
        GENERATOR_EXTENDED
            * Hash::digest_truncated(Domain::Other, &[(*lsk.as_ref()).into()])
                [0],
    );
    let req = Request::new(&pk_lp, &lsa, &k_lic, rng).unwrap();

    // Second, the LP computes these values and grants the License
    let attr_data = JubJubScalar::from(ATTRIBUTE_DATA);
    let lic = License::new(&attr_data, &sk_lp, &req, rng).unwrap();

    let mut tree = LicenseTree::new();
    let lpk = JubJubAffine::from(lic.lsa.note_pk().as_ref());

    let item = LicenseTreeItem {
        hash: Hash::digest(Domain::Other, &[lpk.get_u(), lpk.get_v()])[0],
        data: (),
    };

    let pos = 0;
    tree.insert(pos, item);

    let merkle_proof = tree.opening(pos).expect("Tree was read successfully");

    (lic, merkle_proof)
}

pub fn load_keys(name: impl AsRef<str>) -> Result<(Prover, Verifier), Error> {
    let circuit_profile = rusk_profile::Circuit::from_name(name.as_ref())
        .expect(&format!(
            "the circuit data for {} should be stores",
            name.as_ref()
        ));

    let (pk, vd) = circuit_profile
        .get_keys()
        .expect("The keys for the LicenseCircuit should be stored");

    let prover = Prover::try_from_bytes(&pk)?;
    let verifier = Verifier::try_from_bytes(&vd)?;

    Ok((prover, verifier))
}

#[test]
fn prove_verify_license_circuit() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let (prover, verifier) =
        load_keys("LicenseCircuit").expect("Circuit generation should succeed");

    // user
    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    // license provider
    let sk_lp = SecretKey::random(rng);
    let pk_lp = PublicKey::from(&sk_lp);

    let (lic, merkle_proof) =
        compute_random_license::<StdRng, DEPTH>(rng, &sk, pk, &sk_lp, pk_lp);

    let (cpp, sc) =
        compute_citadel_parameters(rng, &sk, &pk_lp, &lic, merkle_proof);

    let circuit = LicenseCircuit::new(cpp, sc);

    let (proof, public_inputs) = prover
        .prove(rng, &circuit)
        .expect("Proving the circuit should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");
}
