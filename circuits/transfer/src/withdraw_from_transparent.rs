// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_plonk::error::Error as PlonkError;

use dusk_plonk::prelude::*;

#[derive(Debug, Clone)]
pub struct WithdrawFromTransparentCircuit {
    blinder: JubJubScalar,

    // Public data
    commitment: JubJubExtended,
    value: u64,
}

impl WithdrawFromTransparentCircuit {
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

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromTransparentCircuit {
    fn gadget(
        &mut self,
        composer: &mut TurboComposer,
    ) -> Result<(), PlonkError> {
        // Witnesses

        let blinder = composer.append_witness(self.blinder);

        // Public inputs

        let value = composer.append_public_witness(self.value);
        let commitment = composer.append_public_point(self.commitment);

        // Circuit

        // 1. commitment(Nc,Nv,Nb,64)
        gadgets::commitment(composer, commitment, value, blinder, 64);

        Ok(())
    }

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        let value = BlsScalar::from(self.value).into();
        let commitment = self.commitment.into();

        vec![value, commitment]
    }

    fn padded_gates(&self) -> usize {
        1 << 10
    }
}
