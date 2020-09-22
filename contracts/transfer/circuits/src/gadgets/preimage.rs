// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use phoenix_core::note::Note;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED, ExtendedPoint
};
use dusk_plonk::prelude::*;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use dusk_bls12_381::Scalar;
use dusk_pki::Ownable;
use plonk_gadgets::AllocatedScalar;

#[allow(non_snake_case)]
pub fn input_preimage(composer: &mut StandardComposer, 
    value_commitment_x: AllocatedScalar, 
    value_commitment_y: AllocatedScalar, 
    pos: AllocatedScalar, 
    pk_r_x: AllocatedScalar, 
    pk_r_y: AllocatedScalar,
    note_hash: AllocatedScalar,
) {
    let output = sponge_hash_gadget(composer, 
        &[value_commitment_x.var, 
        value_commitment_y.var, 
        pos.var, 
        pk_r_x.var,
        pk_r_y.var,
        ], 
    );

    let zero = composer.add_witness_to_circuit_description(Scalar::zero());

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
    use dusk_pki::PublicSpendKey;
    use phoenix_core::NoteType;

    #[test]
    fn preimage_gadget() {
        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        let value: u64 = rand::thread_rng().gen();
        let sk = Fr::random(&mut rand::thread_rng());
        let v_com = GENERATOR_EXTENDED * sk;
        let v_com_x = AllocatedScalar::allocate(prover.mut_cs(), AffinePoint::from(v_com).get_x());
        let v_com_y = AllocatedScalar::allocate(prover.mut_cs(), AffinePoint::from(v_com).get_y());
        let pos = AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(1));

        let sk = Fr::random(&mut rand::thread_rng());
        let sk2 = Fr::random(&mut rand::thread_rng());
        let pk = GENERATOR_EXTENDED * sk;
        let pk2 = GENERATOR_EXTENDED * sk2;
        
        let note = Note::new(NoteType::Transparent, &PublicSpendKey::new(pk, pk2), value);
        let note_hash = note.hash();
        let note_scalar = AllocatedScalar::allocate(prover.mut_cs(), note_hash);
        let pk_x = AllocatedScalar::allocate(prover.mut_cs(), note.stealth_address().pk_r().get_x());
        let pk_y = AllocatedScalar::allocate(prover.mut_cs(), note.stealth_address().pk_r().get_y());

        input_preimage(prover.mut_cs(), v_com_x, v_com_y, pos, pk_x, pk_y, note_scalar);
        prover.mut_cs().add_dummy_constraints();

        println!("{}", prover.mut_cs().circuit_size());
        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        let v_com_x = AllocatedScalar::allocate(verifier.mut_cs(), AffinePoint::from(v_com).get_x());
        let v_com_y = AllocatedScalar::allocate(verifier.mut_cs(), AffinePoint::from(v_com).get_y());
        let pos = AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(1));
        let note_scalar = AllocatedScalar::allocate(verifier.mut_cs(), note_hash);
        let pk_x = AllocatedScalar::allocate(verifier.mut_cs(), note.stealth_address().pk_r().get_x());
        let pk_y = AllocatedScalar::allocate(verifier.mut_cs(), note.stealth_address().pk_r().get_y());
        input_preimage(verifier.mut_cs(), v_com_x, v_com_y, pos, pk_x, pk_y, note_scalar);
        verifier.mut_cs().add_dummy_constraints();
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}
