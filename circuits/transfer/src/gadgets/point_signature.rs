// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;

/// Prove a given point was signed by a given public key using schnorr single
/// key
pub fn point_signature(
    composer: &mut StandardComposer,
    message: Point,
    pk: Point,
    r: Point,
    u: Variable,
) {
    let message = sponge::gadget(composer, &[*message.x(), *message.y()]);

    schnorr::gadgets::single_key_verify(composer, r, u, pk, message);
}
