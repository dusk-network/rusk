// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::{JubJubAffine, JubJubScalar, GENERATOR_EXTENDED};
use dusk_pki::SecretSpendKey;
use license_circuits::LicenseCircuit;

use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use zk_citadel::license::{
    License, LicenseProverParameters, Request, SessionCookie,
};

mod keys;

// todo: duplication
fn compute_random_license<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> (License, LicenseProverParameters, SessionCookie) {
    // These are the keys of the user
    let ssk = SecretSpendKey::random(rng);
    let psk = ssk.public_spend_key();

    // These are the keys of the LP
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();

    // First, the user computes these values and requests a License
    let lsa = psk.gen_stealth_address(&JubJubScalar::random(rng));
    let k_lic =
        JubJubAffine::from(GENERATOR_EXTENDED * JubJubScalar::from(123456u64));
    let req = Request::new(&psk_lp, &lsa, &k_lic, rng);

    // Second, the LP computes these values and grants the License
    let attr = JubJubScalar::from(112233445566778899u64);
    let lic = License::new(&attr, &ssk_lp, &req, rng);

    // Third, the user computes these values to generate the ZKP later on
    let c = JubJubScalar::from(20221126u64);
    let (lpp, sc) = LicenseProverParameters::compute_parameters(
        &lsa, &ssk, &lic, &psk_lp, &psk_lp, &k_lic, &c, rng,
    );

    (lic, lpp, sc)
}

fn create_random_circuit<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> LicenseCircuit {
    let (_lic, lpp, sc) = compute_random_license(rng);
    LicenseCircuit::new(&lpp, &sc)
}

#[test]
fn prove_verify_license_circuit() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let circuit_id = LicenseCircuit::circuit_id();

    let (prover, verifier) = keys::circuit_keys(circuit_id)
        .expect("Circuit generation should succeed");

    let circuit = create_random_circuit(rng);

    let (proof, public_inputs) = prover
        .prove(rng, &circuit)
        .expect("Proving the circuit should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");
}
