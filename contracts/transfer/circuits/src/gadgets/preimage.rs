// // Copyright (c) DUSK NETWORK. All rights reserved.
// // Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

// use phoenix_core::note::Note;
// use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
// use dusk_plonk::jubjub::{
//     Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
// };
// use dusk_plonk::prelude::*;
// use poseidon252::sponge::sponge::sponge_hash_gadget;
// use dusk_bls12_381::Scalar;
// use dusk_pki::Ownable;

// pub fn input_preimage(composer: &mut StandardComposer, note: &Note) {
    
//     let value_commitment_x = composer.add_input(note.value_commitment().get_x());
//     let value_commitment_y = composer.add_input(note.value_commitment().get_y());

//     let pos =  composer.add_input(Scalar::from(note.pos()));

//     let pk_r_x = AffinePoint::from(note.stealth_address().pk_r()).get_x();
//     let pk_r_y = AffinePoint::from(note.stealth_address().pk_r()).get_y();
//     let R_x = AffinePoint::from(note.stealth_address().R()).get_x();
//     let R_y = AffinePoint::from(note.stealth_address().R()).get_y();

//     let point_x = composer.add_input(pk_r_x);
//     let point_y = composer.add_input(pk_r_y);
//     let rand_x = composer.add_input(R_y);
//     let rand_y = composer.add_input(R_y);
    

//     let output = sponge_hash_gadget(composer, 
//         &[value_commitment_x, 
//         value_commitment_y, 
//         pos, 
//         point_x, 
//         point_y,
//         rand_x,
//         rand_y,
//         ], 
//     );

//     let note_hash = composer.add_input(note.hash());

//     let zero = composer.add_input(Scalar::zero());
    
//     composer.add_gate(
//         output,
//         note_hash,
//         zero, 
//         -BlsScalar::one(),
//         BlsScalar::one(),
//         BlsScalar::one(),

//         BlsScalar::zero(),
//         BlsScalar::zero(),
    
//     );
// }


// #[cfg(test)]
// mod commitment_tests {
//     use super::*;
//     use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
//     use dusk_plonk::proof_system::{Prover, Verifier};
//     use rand::Rng;

//     #[test]
//     fn  preimage_gadget() {
//         let value: u64 = rand::thread_rng().gen();
//         let sk = Fr::random(&mut rand::thread_rng());
//         let pk = AffinePoint::from(GENERATOR_EXTENDED * sk);
//         let note = Note::new(0, pk, value);

//         // Generate Composer & Public Parameters
//         let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
//         let (ck, vk) = pub_params.trim(1 << 16).unwrap();
//         let mut prover = Prover::new(b"test");

//         input_preimage(prover.mut_cs(), &Note);
//         prover.mut_cs().add_dummy_constraints();

//         let circuit = prover.preprocess(&ck).unwrap();
//         let proof = prover.prove(&ck).unwrap();

//         let mut verifier = Verifier::new(b"test");
//         input_preimage(verifier.mut_cs(), &Note);
//         verifier.mut_cs().add_dummy_constraints();
//         verifier.preprocess(&ck).unwrap();
        
//         let pi = verifier.mut_cs().public_inputs.clone();
//         verifier.verify(&proof, &vk, &pi).unwrap();
//     }
// }
