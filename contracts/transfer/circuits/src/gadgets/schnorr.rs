// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED, ExtendedPoint
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use std::convert::TryInto;

/// Verifies that schnorr signature outputs
/// share the same Discrete log
pub fn schnorr(
    composer: &mut StandardComposer,
    signature: AllocatedScalar,
    R: PlonkPoint,
    R_prime: PlonkPoint,
    pk: PlonkPoint,
    pk_prime: PlonkPoint,
    message: AllocatedScalar,
) {
    let h = sponge_hash_gadget(composer, &[message.var]);
    let challenge = sponge_hash_gadget(
        composer,
        &[*R.x(), *R.y(), *R_prime.x(), *R_prime.y(), h],
    );

    let sig_1 = scalar_mul(composer, signature.var, GENERATOR_EXTENDED);
    let sig_2 = scalar_mul(composer, signature.var, GENERATOR_NUMS_EXTENDED);

    let pub_1 = scalar_mul(composer, challenge, ExtendedPoint::from(pk));
    let pub_2 = scalar_mul(composer, challenge, ExtendedPoint::from(pk_prime));



}
