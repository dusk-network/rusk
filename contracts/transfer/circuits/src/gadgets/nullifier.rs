// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use phoenix_core::note::Note;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use dusk_bls12_381::Scalar;
use plonk_gadgets::AllocatedScalar;

pub fn nullifier(composer: &mut StandardComposer, note: &Note, sk: AllocatedScalar, nullifier: AllocatedScalar) {
    
    let pos = note.pos();
    let pos_r = AllocatedScalar::allocate(composer, BlsScalar::from(pos));


    let zero = composer.add_input(Scalar::zero());

    let output = sponge_hash_gadget(composer, &[sk.var, pos_r.var]);


    composer.add_gate(
        output, 
        zero, 
        zero,
        -BlsScalar::one(), 
        BlsScalar::one(), 
        BlsScalar::one(),
        BlsScalar::zero(), 
        nullifier.scalar,
    );
}


// #[cfg(test)]
// mod commitment_tests {
//     use super::*;
//     use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
//     use dusk_plonk::proof_system::{Prover, Verifier};
//     use rand::Rng;

//     #[test]
//     fn  nullifier_gadget() {
//         let value: u64 = rand::thread_rng().gen();

//         // Generate Composer & Public Parameters
//         let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
//         let (ck, vk) = pub_params.trim(1 << 16).unwrap();
//         let mut prover = Prover::new(b"test");

//         nullifier(prover.mut_cs(), value);
//         prover.mut_cs().add_dummy_constraints();

//         let circuit = prover.preprocess(&ck).unwrap();
//         let proof = prover.prove(&ck).unwrap();

//         let mut verifier = Verifier::new(b"test");
//         nullifier(verifier.mut_cs(), value);
//         verifier.mut_cs().add_dummy_constraints();
//         verifier.preprocess(&ck).unwrap();
        
//         let pi = verifier.mut_cs().public_inputs.clone();
//         verifier.verify(&proof, &vk, &pi).unwrap();
//     }
// }

