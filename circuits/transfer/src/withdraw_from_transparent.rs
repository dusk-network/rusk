// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_plonk::error::Error as PlonkError;

use dusk_plonk::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct WithdrawFromTransparentCircuit {
    blinder: JubJubScalar,

    // Public data
    commitment: JubJubExtended,
    value: u64,
}

impl WithdrawFromTransparentCircuit {
    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }

    pub const fn new(
        commitment: JubJubExtended,
        value: u64,
        blinder: JubJubScalar,
    ) -> Self {
        Self {
            commitment,
            value,
            blinder,
        }
    }
}

#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for WithdrawFromTransparentCircuit {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        // Witnesses

        let blinder = composer.append_witness(self.blinder);

        // Public inputs

        let value = composer.append_public(self.value);
        let commitment = composer.append_public_point(self.commitment);

        // Circuit

        // 1. commitment(Nc,Nv,Nb,64)
        gadgets::commitment(composer, commitment, value, blinder, 64)?;

        Ok(())
    }
}
