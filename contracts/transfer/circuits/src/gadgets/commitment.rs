// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};

use dusk_plonk::prelude::*;

/// Prove knowledge of the value and blinding factor, which make up the value
/// commitment. This commitment gadget is using the pedersen commitments.
/// C = a*g + b*h
pub fn commitment(
    composer: &mut StandardComposer,
    value: Variable,
    blinder: Variable,
) -> Point {
    let p1 = scalar_mul(composer, value, GENERATOR_EXTENDED);
    let p2 = scalar_mul(composer, blinder, GENERATOR_NUMS_EXTENDED);

    p1.point().fast_add(composer, *p2.point())
}
