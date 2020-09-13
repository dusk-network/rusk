// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

// Prove that the amount inputted equals the amount outputted
pub fn balance(composer: &mut StandardComposer, v_in: u64, v_out: u64) {
    let s = v_in - v_out;
    composer.constrain_to_constant(Scalar::from(s), Scalar::zero(), Scalar::zero());
}



