// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

// use phoenix_core::{Note, TransactionItem, ViewKey};

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use rand::*;
use plonk_gadgets::AllocatedScalar;
use dusk_bls12_381::Scalar;

// pub struct PedersenCommitment {
//     r: ExtendedPoint
//     c: ExtendedPoint
// }

/// Prove knowledge of the value and blinding factor, which make up the value commitment.
/// This commitment gadget is using the pedersen commitments.
pub fn commitment(composer: &mut StandardComposer, value: Fr, blinder: Fr, pub_commit: AffinePoint) {
    let zero = composer.add_witness_to_circuit_description(Scalar::zero());
    let bid_value = AllocatedScalar::allocate(composer, value.into());
    let blinder = composer.add_input(blinder.into());

    let p1 = scalar_mul(composer, bid_value.var, GENERATOR_EXTENDED);
    let p2 = scalar_mul(composer, blinder, GENERATOR_NUMS_EXTENDED);

    let commitment = p1.point().fast_add(composer, *p2.point());

    composer.assert_equal_public_point(commitment, pub_commit);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn commitment_gadget() {
        let value = Fr::from(100 as u64);
        let blinder = Fr::from(20000 as u64);
        let p1 = GENERATOR_EXTENDED * value;
        let p2 = GENERATOR_NUMS_EXTENDED * blinder;

        let pc_commitment = p1 + p2;

        let mut prover = Prover::new(b"test");

        commitment(prover.mut_cs(), value, blinder, AffinePoint::from(pc_commitment));
        // prover.mut_cs().add_dummy_constraints();
        // NOTE: this is here to make the test pass, as one set of dummy constraints
        // isn't enough when no extra gates are added. It should be removed once the
        // commitment gadget is properly implemented.
        // prover.mut_cs().add_dummy_constraints();
        ////////////////////////////////////////////////////////////////////////////

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        commitment(verifier.mut_cs(), value, blinder, AffinePoint::from(pc_commitment));
        verifier.preprocess(&ck).unwrap();
        prover.mut_cs().check_circuit_satisfied();
        verifier.verify(&proof, &vk, &prover.mut_cs().public_inputs.clone()).unwrap();
    }
}
