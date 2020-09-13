// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

// use phoenix_core::{Note, TransactionItem, ViewKey};

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;



// pub struct PedersenCommitment {
//     r: ExtendedPoint
//     c: ExtendedPoint
// }

/// Prove knowledge of the value and blinding factor, which make up the value commitment.
/// This commitment gadget is using the pedersen commitments.
pub fn commitment(composer: &mut StandardComposer, value: Variable, blinder: Variable, pub_commit: AffinePoint) {

    let p1 = scalar_mul(composer, value, GENERATOR_EXTENDED);
    let p2 = scalar_mul(composer, blinder, GENERATOR_NUMS_EXTENDED);

    let commitment = p1.point().fast_add(composer, *p2.point());

    composer.assert_equal_public_point(commitment, pub_commit);

}


#[cfg(test)]
mod commitment_tests {
    use super::*;
    use merlin::Transcript;

    #[test]
    fn commitment_gadget() {
        let value = Fr::from(100);
        let blinder = Fr::from(20000);
        let p1 = GENERATOR_EXTENDED * value;
        let p2 = GENERATOR_NUMS_EXTENDED * blinder;

        let pc_commitment = p1 + p2;

        let mut composer = StandardComposer::new();
        let value_var = composer.add_input(value.into());
        let blinder_var = composer.add_input(blinder.into());

        commitment(&mut composer, value_var, blinder_var, AffinePoint::from(pc_commitment));
        composer.add_dummy_constraints();
        // NOTE: this is here to make the test pass, as one set of dummy constraints
        // isn't enough when no extra gates are added. It should be removed once the
        // commitment gadget is properly implemented.
        composer.add_dummy_constraints();
        ////////////////////////////////////////////////////////////////////////////

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut transcript = Transcript::new(b"TEST");

        let circuit = composer.preprocess(
            &ck,
            &mut transcript,
            &EvaluationDomain::new(composer.circuit_size()).unwrap(),
        );

        let proof = composer.prove(&ck, &circuit, &mut transcript.clone());

        assert!(proof.verify(&circuit, &mut transcript, &vk, &composer.public_inputs));
    }
}

