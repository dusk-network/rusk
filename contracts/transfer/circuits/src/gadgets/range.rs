// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

/// This gadget simply wraps around the composer's `range_gate` function,
/// but takes in any type that implements the traits of the note,
/// for ease-of-use in circuit construction.
///
/// It will check the range boundaries for expected u64 values
pub fn range(composer: &mut StandardComposer, value: Variable) {
    composer.range_gate(value, 64);
}
