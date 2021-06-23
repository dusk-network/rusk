// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_plonk::constraint_system::ecc::Point;
use dusk_poseidon::sponge::truncated;

use dusk_plonk::prelude::*;

/// Prove knowledge of the value and blinding factor, which make up the value
/// commitment. This commitment gadget is using the pedersen commitments.
/// C = a*g + b*h
pub fn commitment(
    composer: &mut StandardComposer,
    value: Variable,
    blinder: Variable,
) -> Point {
    let p1 = composer.fixed_base_scalar_mul(value, GENERATOR_EXTENDED);
    let p2 = composer.fixed_base_scalar_mul(blinder, GENERATOR_NUMS_EXTENDED);

    composer.point_addition_gate(p1, p2)
}

/// Conditionally select a point against identity
///
/// Returns:
/// a, if flag == 1 ^ b == identity
/// b, if flag == 0 ^ a == identity
///
/// Fail the circuit otherwise
pub fn identity_select_point(
    composer: &mut StandardComposer,
    flag: Variable,
    identity: Point,
    a: Point,
    b: Point,
) -> Point {
    let flag = composer.boolean_gate(flag);

    let selected = composer.conditional_point_select(a, b, flag);
    let discarded = composer.conditional_point_select(b, a, flag);

    composer.assert_equal_point(discarded, identity);

    selected
}

/// Derives a stealth address out of a public spend key
///
/// S = H(r · A) · G + B
pub fn stealth_address(
    composer: &mut StandardComposer,
    r: Variable,
    a: Point,
    b: Point,
) -> Point {
    let a = composer.variable_base_scalar_mul(r, a);
    let a = truncated::gadget(composer, &[*a.x(), *a.y()]);
    let a = composer.fixed_base_scalar_mul(a, GENERATOR_EXTENDED);

    composer.point_addition_gate(a, b)
}
