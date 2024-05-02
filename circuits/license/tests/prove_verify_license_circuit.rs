// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use license_circuits::{Error, LicenseCircuit, ARITY, DEPTH};
use phoenix_core::{PublicKey, SecretKey};

use rand::rngs::StdRng;
use rand::SeedableRng;

use zk_citadel::utils::CitadelUtils;

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
        CitadelUtils::compute_random_license::<StdRng, DEPTH, ARITY>(
            rng, sk, pk, sk_lp, pk_lp,
        );

    let (cpp, sc) = CitadelUtils::compute_citadel_parameters::<
        StdRng,
        DEPTH,
        ARITY,
    >(rng, sk, pk_lp, &lic, merkle_proof);

    let circuit = LicenseCircuit::new(cpp, sc);

    let (proof, public_inputs) = prover
        .prove(rng, &circuit)
        .expect("Proving the circuit should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Verifying the circuit should succeed");
}
