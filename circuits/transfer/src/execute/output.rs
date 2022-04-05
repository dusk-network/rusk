// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::Note;

use dusk_plonk::prelude::*;

use crate::ExecuteCircuit;

#[derive(Debug, Clone)]
pub struct CircuitOutput {
    note: Note,
    value: u64,
    blinding_factor: JubJubScalar,
}

impl CircuitOutput {
    /// Zeroed stealth address used for padding
    pub const ZERO_STEALTH_ADDRESS: StealthAddress = {
        let pk = ExecuteCircuit::ZERO_COMMITMENT;
        let pk = PublicKey::from_raw_unchecked(pk);

        let r = ExecuteCircuit::ZERO_COMMITMENT;

        StealthAddress::from_raw_unchecked(r, pk)
    };

    pub fn new(note: Note, value: u64, blinding_factor: JubJubScalar) -> Self {
        Self {
            note,
            value,
            blinding_factor,
        }
    }

    pub const fn note(&self) -> &Note {
        &self.note
    }

    pub const fn value_commitment(&self) -> &JubJubExtended {
        self.note.value_commitment()
    }

    pub fn pad() -> Self {
        let note = Note::transparent_stealth(
            Self::ZERO_STEALTH_ADDRESS,
            0,
            BlsScalar::zero(),
        );

        Self::new(note, 0, JubJubScalar::zero())
    }

    pub fn to_witness(&self, composer: &mut TurboComposer) -> WitnessOutput {
        let value = composer.append_witness(self.value);
        let blinding_factor = composer.append_witness(self.blinding_factor);
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
    pub value: Witness,
    pub blinding_factor: Witness,

    // Public data
    pub value_commitment: JubJubExtended,
}
