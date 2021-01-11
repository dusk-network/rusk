// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::prelude::*;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use schnorr::gadgets as schnorr_gadgets;

/// Prove a given point was signed by a given public key using schnorr single key
pub fn point_signature(
    composer: &mut StandardComposer,
    message: Point,
    pk: Point,
    r: Point,
    u: Variable,
) {
    let message = sponge_hash_gadget(composer, &[*message.x(), *message.y()]);

    schnorr_gadgets::single_key_verify(composer, r, u, pk, message);
}
