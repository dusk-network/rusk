// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

/// This gadget simply wraps around the composer's `range_gate` function,
/// but takes in any type that implements the [`TransactionItem`] trait,
/// for ease-of-use in circuit construction.
pub fn range(composer: &mut StandardComposer, value: u64) {
    let value = composer.add_input(BlsScalar::from(value));
    
    composer.range_gate(value, 64);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;

    #[test]
    fn  range_gadget() {
        let value: u64 = rand::thread_rng().gen();

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        range(prover.mut_cs(), value);
        prover.mut_cs().add_dummy_constraints();

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        range(verifier.mut_cs(), value);
        verifier.mut_cs().add_dummy_constraints();
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}