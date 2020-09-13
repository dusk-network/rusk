// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

// Prove that the amount inputted equals the amount outputted
pub fn secret_knowledge(composer: &mut StandardComposer, sk: Fr, pk: AffinePoint) {
    
    let p1 = AffinePoint::from(scalar_mul(composer, value, GENERATOR_EXTENDED));

    composer.assert_equal_public_point(p1, pk);
}
