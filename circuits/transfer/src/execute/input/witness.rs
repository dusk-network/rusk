// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

/// Coupled code
///
/// Currently, Plonk is not a dependency of phoenix-core. This means the circuit
/// construction of the note must be done here.
///
/// Ideally, there would be a `fn hash_inputs_witness(&self, composer)`
/// implemented for `Note`.
///
/// Since the circuit will perform a pre-image check over the result of this
/// function, the structure is safe
///
/// However, if `Note::hash_inputs` ever change, this circuit will be broken
#[derive(Debug, Clone, Copy)]
pub struct WitnessInput {
    pub note_type: Witness,
    pub value: Witness,
    pub blinding_factor: Witness,
    pub value_commitment: WitnessPoint,
    pub pk_r: WitnessPoint,
    pub pk_r_p: WitnessPoint,
    pub schnorr_u: Witness,
    pub schnorr_r: WitnessPoint,
    pub schnorr_r_p: WitnessPoint,
    pub pos: Witness,
    pub note_hash: Witness,
    pub nullifier: BlsScalar,
}

impl WitnessInput {
    #[allow(clippy::wrong_self_convention)]
    pub fn to_hash_inputs(&self) -> [Witness; 6] {
        [
            self.note_type,
            *self.value_commitment.x(),
            *self.value_commitment.y(),
            *self.pk_r.x(),
            *self.pk_r.y(),
            self.pos,
        ]
    }
}
