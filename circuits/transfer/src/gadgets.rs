// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_merkle::Aggregate;
use dusk_plonk::prelude::*;
use dusk_poseidon::cipher;
use dusk_poseidon::sponge::truncated;
use poseidon_merkle::{zk::opening_gadget, Opening};

pub use jubjub_schnorr::gadgets::verify_signature as schnorr_single_key_verify;
pub use jubjub_schnorr::gadgets::verify_signature_double as schnorr_double_key_verify;

/// Prove the opening of a Pedersen commitment and prove that `v` is in the
/// range of `2^bits`.
///
/// `commitment(p, v, b, s) → p == v · G + b · G′ ∧ v < 2^s`
pub fn commitment(
    composer: &mut Composer,
    p: WitnessPoint,
    v: Witness,
    b: Witness,
) -> Result<(), Error> {
    const HALF_64: usize = 32;
    composer.component_range::<HALF_64>(v);

    let v = composer.component_mul_generator(v, GENERATOR_EXTENDED)?;
    let b = composer.component_mul_generator(b, GENERATOR_NUMS_EXTENDED)?;

    let p_p = composer.component_add_point(v, b);

    composer.assert_equal_point(p, p_p);

    Ok(())
}

/// Prove the merkle opening of the branch and assert that anchor and leaf
/// matches.
///
/// `opening(b, r, l) → O(b) ∧ (b0, b|b|) == (l, r)`
pub fn merkle_opening<T, const H: usize, const A: usize>(
    composer: &mut Composer,
    branch: &Opening<T, H, A>,
    anchor: Witness,
    leaf: Witness,
) where
    T: Clone + Aggregate<A>,
{
    // The gadget asserts the leaf is the expected
    let root = opening_gadget(composer, branch, leaf);
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
    composer: &mut Composer,
    flag: Witness,
    identity: WitnessPoint,
    a: WitnessPoint,
    b: WitnessPoint,
) -> WitnessPoint {
    composer.component_boolean(flag);

    let selected = composer.component_select_point(flag, a, b);
    let discarded = composer.component_select_point(flag, b, a);

    composer.assert_equal_point(discarded, identity);

    selected
}

/// Derives a stealth address out of a public spend key
///
/// S = H(r · A) · G + B
pub fn stealth_address(
    composer: &mut Composer,
    r: Witness,
    a: WitnessPoint,
    b: WitnessPoint,
) -> Result<WitnessPoint, Error> {
    let a = composer.component_mul_point(r, a);
    let a = truncated::gadget(composer, &[*a.x(), *a.y()]);
    let a = composer.component_mul_generator(a, GENERATOR_EXTENDED)?;

    Ok(composer.component_add_point(a, b))
}

pub fn encrypt(
    composer: &mut Composer,
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
