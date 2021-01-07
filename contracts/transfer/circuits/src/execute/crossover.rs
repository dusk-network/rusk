// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::jubjub::JubJubExtended;
use dusk_plonk::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct CircuitCrossover {
    value_commitment: JubJubExtended,
    value: u64,
    blinding_factor: JubJubScalar,
}

impl CircuitCrossover {
    pub fn new(
        value_commitment: JubJubExtended,
        value: u64,
        blinding_factor: JubJubScalar,
    ) -> Self {
        Self {
            value_commitment,
            value,
            blinding_factor,
        }
    }

    pub fn to_witness(
        &self,
        composer: &mut StandardComposer,
    ) -> WitnessCrossover {
        let value_commitment = self.value_commitment;

        let value = BlsScalar::from(self.value);
        let fee_value = value;
        let value = composer.add_input(value);

        let blinding_factor = BlsScalar::from(self.blinding_factor);
        let blinding_factor = composer.add_input(blinding_factor);

        WitnessCrossover {
            value,
            blinding_factor,
            fee_value,
            value_commitment,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessCrossover {
    pub value: Variable,
    pub blinding_factor: Variable,

    // Public data
    pub fee_value: BlsScalar,
    pub value_commitment: JubJubExtended,
}
