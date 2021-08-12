// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_jubjub::{JubJubAffine, JubJubExtended};
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::prelude::*;
use phoenix_core::{Error as PhoenixError, Message, Note};

#[derive(Debug, Clone)]
pub struct WithdrawFromObfuscatedCircuit {
    message_value: u64,
    message_blinder: JubJubScalar,

    message_commitment: JubJubExtended,

    output_value: u64,
    output_blinder: JubJubScalar,
    output_commitment: JubJubExtended,
}

impl WithdrawFromObfuscatedCircuit {
    pub fn new(
        message_r: JubJubScalar,
        message_ssk: &SecretSpendKey,
        message: &Message,
        output: &Note,
        output_vk: Option<&ViewKey>,
    ) -> Result<Self, PhoenixError> {
        let message_psk = message_ssk.public_spend_key();

        let message_commitment = *message.value_commitment();
        let (message_value, message_blinder) =
            message.decrypt(&message_r, &message_psk)?;

        let output_value = output.value(output_vk)?;
        let output_blinder = output.blinding_factor(output_vk)?;
        let output_commitment = *output.value_commitment();

        Ok(Self {
            message_value,
            message_blinder,

            message_commitment,

            output_value,
            output_blinder,
            output_commitment,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        // 1. commitment(Mc,mv,mb)
        let message_commitment = JubJubAffine::from(self.message_commitment);

        // 3. commitment(Oc,ov,ob)
        let output_commitment = JubJubAffine::from(self.output_commitment);

        vec![message_commitment.into(), output_commitment.into()]
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        let mv = composer.add_input(self.message_value.into());
        let mb = composer.add_input(self.message_blinder.into());

        let ov = composer.add_input(self.output_value.into());
        let ob = composer.add_input(self.output_blinder.into());

        // 1. commitment(Mc,mv,mb)
        let commitment = gadgets::commitment(composer, mv, mb);
        composer.assert_equal_public_point(
            commitment,
            self.message_commitment.into(),
        );

        // 2. range(mv,64)
        composer.range_gate(mv, 64);

        // 3. commitment(Oc,ov,ob)
        let commitment = gadgets::commitment(composer, ov, ob);
        composer.assert_equal_public_point(
            commitment,
            self.output_commitment.into(),
        );

        // 4. range(mv,64)
        composer.range_gate(ov, 64);

        // 5. mv - nv = 0
        composer.poly_gate(
            mv,
            mv,
            ov,
            BlsScalar::zero(),
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            None,
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 11
    }
}
