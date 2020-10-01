// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{AffinePoint, GENERATOR_EXTENDED};
use dusk_plonk::prelude::*;

use plonk_gadgets::AllocatedScalar;

/// Prove knowledge of a secret key, by performing a scalar
/// multiplication in-circuit.
pub fn sk_knowledge(
    composer: &mut StandardComposer,
    sk: AllocatedScalar,
    pk: PlonkPoint,
) {
    
    let p1 = scalar_mul(composer, sk.var, GENERATOR_EXTENDED);
    composer.assert_equal_point(*p1.point(), pk);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn sk_gadget() -> Result<(), Error> {
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let pk = AffinePoint::from(GENERATOR_EXTENDED * sk);
        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 10, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 9)?;
        let mut prover = Prover::new(b"test");

        let sk_r =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(sk));

        let ppk = PlonkPoint::from_private_affine(prover.mut_cs(), pk);

        sk_knowledge(prover.mut_cs(), sk_r, ppk);

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let sk_r =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(sk));

        let ppk = PlonkPoint::from_private_affine(verifier.mut_cs(), pk);

        sk_knowledge(verifier.mut_cs(), sk_r, ppk);
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
