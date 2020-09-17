// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.
/*
use phoenix_core::note::Note;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use dusk_bls12_381::Scalar;

pub fn input_preimage(composer: &mut StandardComposer, note: &Note) {
    
    let value_commitment_x = composer.add_input(note.value_commitment().get_x());
    let value_commitment_y = composer.add_input(note.value_commitment().get_y());

    let idx =  composer.add_input(Scalar::from(note.pos()));

    let pk_r_x = composer.add_input(note.stealth_address().pk_r().get_x());
    let pk_r_y = composer.add_input(note.stealth_address().pk_r().get_y());

    let output = sponge_hash_gadget(composer, 
        &[value_commitment_x, 
        value_commitment_y, 
        idx, 
        pk_r_x, 
        pk_r_y,
        ], 
    );

    let note_hash = composer.add_input(note.hash());

    let zero = composer.add_input(Scalar::zero());
    
    composer.add_gate(
        output,
        note_hash,
        zero, 
        -BlsScalar::one(),
        BlsScalar::one(),
        BlsScalar::one(),

        BlsScalar::zero(),
        BlsScalar::zero(),
    
    );
}
*/