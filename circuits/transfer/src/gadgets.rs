// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::POSEIDON_TREE_DEPTH;

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_poseidon::cipher;
use dusk_poseidon::sponge::truncated;
use dusk_poseidon::tree::{self, PoseidonBranch};

use dusk_plonk::prelude::*;

pub use dusk_schnorr::gadgets::double_key_verify as schnorr_double_key_verify;
pub use dusk_schnorr::gadgets::single_key_verify as schnorr_single_key_verify;

/// Prove the opening of a Pedersen commitment and prove that `v` is in the
/// range of `2^bits`.
///
/// `commitment(p, v, b, s) → p == v · G + b · G′ ∧ v < 2^s`
pub fn commitment(
    composer: &mut TurboComposer,
    p: WitnessPoint,
    v: Witness,
    b: Witness,
    bits: usize,
) {
    composer.component_range(v, bits);

    let v = composer.component_mul_generator(v, GENERATOR_EXTENDED);
    let b = composer.component_mul_generator(b, GENERATOR_NUMS_EXTENDED);

    let p_p = composer.component_add_point(v, b);

    composer.assert_equal_point(p, p_p);
}

/// Prove the merkle opening of the branch and assert that anchor and leaf
/// matches.
///
/// `opening(b, r, l) → O(b) ∧ (b0, b|b|) == (l, r)`
pub fn merkle_opening(
    composer: &mut TurboComposer,
    branch: &PoseidonBranch<POSEIDON_TREE_DEPTH>,
    anchor: Witness,
    leaf: Witness,
) {
    // The gadget asserts the leaf is the expected
    let root = tree::merkle_opening(composer, branch, leaf);

    composer.assert_equal(anchor, root);
}

/// Conditionally select a point against identity
///
/// Returns:
/// a, if flag == 1 ^ b == identity
/// b, if flag == 0 ^ a == identity
///
/// Fail the circuit otherwise
pub fn identity_select_point(
    composer: &mut TurboComposer,
    flag: Witness,
    identity: WitnessPoint,
    a: WitnessPoint,
    b: WitnessPoint,
) -> WitnessPoint {
    composer.component_boolean(flag);

    let selected = composer.component_select_point(a, b, flag);
    let discarded = composer.component_select_point(b, a, flag);

    composer.assert_equal_point(discarded, identity);

    selected
}

/// Derives a stealth address out of a public spend key
///
/// S = H(r · A) · G + B
pub fn stealth_address(
    composer: &mut TurboComposer,
    r: Witness,
    a: WitnessPoint,
    b: WitnessPoint,
) -> WitnessPoint {
    let a = composer.component_mul_point(r, a);
    let a = truncated::gadget(composer, &[*a.x(), *a.y()]);
    let a = composer.component_mul_generator(a, GENERATOR_EXTENDED);

    composer.component_add_point(a, b)
}

pub fn encrypt(
    composer: &mut TurboComposer,
    secret: WitnessPoint,
    nonce: Witness,
    message: &[Witness],
    cipher: &[Witness],
) {
    let cipher_p = cipher::encrypt(composer, &secret, nonce, message);

    cipher
        .iter()
        .zip(cipher_p.iter())
        .for_each(|(c, p)| composer.assert_equal(*c, *p));
}
