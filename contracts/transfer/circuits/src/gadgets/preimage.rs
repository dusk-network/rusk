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
use dusk_pki::Ownable;
use plonk_gadgets::AllocatedScalar;

pub fn input_preimage(composer: &mut StandardComposer, note: &Note) {
    let value_commitment_x = AllocatedScalar::allocate(composer, note.value_commitment().get_x());
    let value_commitment_y = AllocatedScalar::allocate(composer, note.value_commitment().get_y());

    let pos = AllocatedScalar::allocate(composer, Scalar::from(note.pos()));

    let pk_r_x = AffinePoint::from(note.stealth_address().pk_r()).get_x();
    let pk_r_y = AffinePoint::from(note.stealth_address().pk_r()).get_y();
    // XXX: These need to be uncommented once the `hash` method for the 
    // phoenix_core::Note is fixed.
    // let R_x = AffinePoint::from(note.stealth_address().R()).get_x();
    // let R_y = AffinePoint::from(note.stealth_address().R()).get_y();

    let point_x = AllocatedScalar::allocate(composer, pk_r_x);
    let point_y = AllocatedScalar::allocate(composer, pk_r_y);
    // let rand_x = AllocatedScalar::allocate(composer, R_x);
    // let rand_y = AllocatedScalar::allocate(composer, R_x);

    let output = sponge_hash_gadget(composer, 
        &[value_commitment_x.var, 
        value_commitment_y.var, 
        pos.var, 
        point_x.var,
        point_y.var,
        // rand_x.var,
        // rand_y.var,
        ], 
    );

    let note_hash = AllocatedScalar::allocate(composer, note.hash());
    let zero = composer.add_input(Scalar::zero());
    println!("{:?}", output);
    println!("{:?}", note_hash);
    
    composer.add_gate(
        output,
        note_hash.var,
        zero, 
        -BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::zero(),
        BlsScalar::zero(),
    );
}


#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;
    use phoenix_core::note::NoteType;
    use dusk_pki::PublicSpendKey;

    #[test]
    fn preimage_gadget() {
        let value: u64 = rand::thread_rng().gen();
        let sk = Fr::random(&mut rand::thread_rng());
        let sk2 = Fr::random(&mut rand::thread_rng());
        let pk = GENERATOR_EXTENDED * sk;
        let pk2 = GENERATOR_EXTENDED * sk2;
        let note = Note::new(NoteType::Transparent, &PublicSpendKey::new(pk, pk2), value);

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        input_preimage(prover.mut_cs(), &note);
        prover.mut_cs().add_dummy_constraints();

        println!("{}", prover.mut_cs().circuit_size());
        // prover.mut_cs().check_circuit_satisfied();
        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        input_preimage(verifier.mut_cs(), &note);
        verifier.mut_cs().add_dummy_constraints();
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}
