// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::jubjub::JubJubExtended;
use phoenix_core::Note;

use dusk_plonk::prelude::*;

#[derive(Debug, Clone)]
pub struct CircuitOutput {
    note: Note,
    value: u64,
    blinding_factor: JubJubScalar,
}

impl CircuitOutput {
    pub fn new(note: Note, value: u64, blinding_factor: JubJubScalar) -> Self {
        Self {
            note,
            value,
            blinding_factor,
        }
    }

    pub fn value_commitment(&self) -> &JubJubExtended {
        self.note.value_commitment()
    }

    pub fn to_witness(&self, composer: &mut StandardComposer) -> WitnessOutput {
        let value = composer.add_input(self.value.into());
        let blinding_factor = composer.add_input(self.blinding_factor.into());
        let value_commitment = *self.note.value_commitment();

        WitnessOutput {
            value,
            blinding_factor,
            value_commitment,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessOutput {
    pub value: Variable,
    pub blinding_factor: Variable,

    // Public data
    pub value_commitment: JubJubExtended,
}
