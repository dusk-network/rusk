// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_pki::ViewKey;
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended};
use dusk_plonk::prelude::*;
use phoenix_core::{Error as PhoenixError, Note};

#[derive(Debug, Clone)]
pub struct WithdrawFromTransparentCircuit {
    blinding_factor: JubJubScalar,

    // Public data
    value_commitment: JubJubExtended,
    value: BlsScalar,
}

impl WithdrawFromTransparentCircuit {
    pub fn new(
        note: &Note,
        vk: Option<&ViewKey>,
    ) -> Result<Self, PhoenixError> {
        let value_commitment = *note.value_commitment();
        let value = note.value(vk)?.into();
        let blinding_factor = note.blinding_factor(vk)?;

        Ok(Self {
            blinding_factor,
            value_commitment,
            value,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let value = self.value;
        let value_commitment = JubJubAffine::from(self.value_commitment);

        vec![value.into(), value_commitment.into()]
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromTransparentCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        // 1. Prove the knowledge of the commitment
        let value = composer.add_input(self.value.into());
        composer.constrain_to_constant(
            value,
            BlsScalar::zero(),
            Some(-self.value),
        );

        let blinding_factor = self.blinding_factor.into();
        let blinding_factor = composer.add_input(blinding_factor);

        let value_commitment_p =
            gadgets::commitment(composer, value, blinding_factor);

        let value_commitment = JubJubAffine::from(self.value_commitment);
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        // 2. Prove that the value is within range
        composer.range_gate(value, 64);

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 10
    }
}
