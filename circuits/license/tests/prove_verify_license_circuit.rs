// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use license_circuits::LicenseCircuit;

use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};

use zk_citadel::license::License;


mod keys;

fn create_random_circuit<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> LicenseCircuit {
    let license = License::random(rng);
    LicenseCircuit::new(license)
}

#[test]
fn prove_verify_license_circuit() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let circuit_id = LicenseCircuit::circuit_id();

    let (prover, verifier) =
        keys::circuit_keys(circuit_id).expect("Circuit generation should succeed");

    let circuit = create_random_circuit(rng);

    let (proof, public_inputs) = prover
        .prove(rng, &circuit)
        .expect("Proving the circuit should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");
}
