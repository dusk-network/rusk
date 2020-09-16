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

pub fn nullifier(composer: &mut StandardComposer, note: &Note, sk: Scalar, nullifier: Scalar) {
    
    let sk_r = composer.add_input(sk);
    
    let idx =  composer.add_input(Scalar::from(note.pos()));

    let zero = composer.add_input(Scalar::zero());

    let output = sponge_hash_gadget(composer, &[sk_r, idx]);

    composer.add_gate(
        output, 
        zero, 
        zero, 
        -BlsScalar::one(), 
        BlsScalar::one(), 
        BlsScalar::one(), 
        BlsScalar::zero(), 
        nullifier,
    );
}