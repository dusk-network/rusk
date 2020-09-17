// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_bls12_381::Scalar;
use dusk_plonk::prelude::*;

// Prove that the amount inputted equals the amount outputted
pub fn balance(composer: &mut StandardComposer, v_in: u64, v_out: u64) {
    let s = v_in - v_out;
    let s_var = composer.add_input(Scalar::from(s));
    composer.constrain_to_constant(s_var, Scalar::zero(), Scalar::zero());
}



#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn  balance_gadget() {
        let v_in = 100 as u64;
        let v_out = 100 as u64;


        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        balance(prover.mut_cs(), v_in, v_out);
        prover.mut_cs().add_dummy_constraints();

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        balance(verifier.mut_cs(), v_in, v_out);
        verifier.mut_cs().add_dummy_constraints();
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}