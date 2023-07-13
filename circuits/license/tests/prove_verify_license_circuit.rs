// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::SecretSpendKey;
use license_circuits::{LicenseCircuit, ARITY, DEPTH};

use rand::rngs::StdRng;
use rand::SeedableRng;

use zk_citadel::utils::CitadelUtils;
mod keys;

#[test]
fn prove_verify_license_circuit() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let circuit_id = LicenseCircuit::circuit_id();

    let (prover, verifier) = keys::circuit_keys(circuit_id)
        .expect("Circuit generation should succeed");

    // user
    let ssk = SecretSpendKey::random(rng);
    let psk = ssk.public_spend_key();

    // license provider
    let ssk_lp = SecretSpendKey::random(rng);
    let psk_lp = ssk_lp.public_spend_key();

    let (cpp, sc) =
        CitadelUtils::compute_citadel_parameters::<StdRng, DEPTH, ARITY>(
            rng, ssk, psk, ssk_lp, psk_lp,
        );
    let circuit = LicenseCircuit::new(&cpp, &sc);

    let (proof, public_inputs) = prover
        .prove(rng, &circuit)
        .expect("Proving the circuit should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");
}
