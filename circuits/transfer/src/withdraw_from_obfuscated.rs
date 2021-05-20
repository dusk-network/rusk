// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended};
use dusk_plonk::prelude::*;
use dusk_poseidon::cipher::{self, PoseidonCipher};
use phoenix_core::{Error as PhoenixError, Message, Note};

#[derive(Debug, Clone)]
pub struct WithdrawFromObfuscatedCircuit {
    input_value: u64,
    input_blinding_factor: JubJubScalar,
    change_r: JubJubScalar,
    change_value: u64,
    change_blinding_factor: JubJubScalar,
    output_value: u64,
    output_blinding_factor: JubJubScalar,

    // Public data
    input_value_commitment: JubJubExtended,
    change_value_commitment: JubJubExtended,
    change_nonce: JubJubScalar,
    change_cipher: [BlsScalar; PoseidonCipher::cipher_size()],
    change_pk: JubJubExtended,
    output_value_commitment: JubJubExtended,
}

impl WithdrawFromObfuscatedCircuit {
    pub const fn rusk_keys_id() -> &'static str {
        "transfer-withdraw-from-obfuscated"
    }

    pub fn new(
        input: &Note,
        input_view_key: Option<&ViewKey>,
        change: &Message,
        change_r: JubJubScalar,
        change_psk: &PublicSpendKey,
        output: &Note,
        output_view_key: Option<&ViewKey>,
    ) -> Result<Self, PhoenixError> {
        let input_value = input.value(input_view_key)?;
        let input_blinding_factor = input.blinding_factor(input_view_key)?;
        let input_value_commitment = *input.value_commitment();

        let change_pk = *change_psk.A();
        let change_value_commitment = *change.value_commitment();
        let change_nonce = *change.nonce();
        let change_cipher = *change.cipher();
        let (change_value, change_blinding_factor) =
            change.decrypt(&change_r, &change_psk)?;

        let output_value = output.value(output_view_key)?;
        let output_blinding_factor = output.blinding_factor(output_view_key)?;
        let output_value_commitment = *output.value_commitment();

        Ok(Self {
            input_value,
            input_blinding_factor,
            input_value_commitment,
            change_r,
            change_value,
            change_blinding_factor,
            change_value_commitment,
            change_nonce,
            change_cipher,
            change_pk,
            output_value,
            output_blinding_factor,
            output_value_commitment,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = vec![];

        // step 1
        let input_value_commitment =
            JubJubAffine::from(self.input_value_commitment);
        pi.push(input_value_commitment.into());

        // step 3
        let change_value_commitment =
            JubJubAffine::from(self.change_value_commitment);
        pi.push(change_value_commitment.into());

        // step 7
        pi.push(self.change_nonce.into());

        let change_pk = JubJubAffine::from(self.change_pk);
        pi.push(change_pk.into());

        pi.extend(self.change_cipher.iter().map(|c| (*c).into()));

        // step 8
        let output_value_commitment =
            JubJubAffine::from(self.output_value_commitment);
        pi.push(output_value_commitment.into());

        pi
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        // 1. Prove the knowledge of the commitment opening of the commitment of
        // the input
        let input_value = composer.add_input(self.input_value.into());

        let input_blinding_factor = self.input_blinding_factor.into();
        let input_blinding_factor = composer.add_input(input_blinding_factor);

        let input_value_commitment_p =
            gadgets::commitment(composer, input_value, input_blinding_factor);

        let input_value_commitment = self.input_value_commitment.into();
        composer.assert_equal_public_point(
            input_value_commitment_p,
            input_value_commitment,
        );

        // 2. Prove that the value of the opening of the commitment of the input
        // is within range
        composer.range_gate(input_value, 64);

        // 3. Prove the knowledge of the commitment opening of the commitment of
        // the change
        let change_value = composer.add_input(self.change_value.into());

        let change_blinding_factor = self.change_blinding_factor.into();
        let change_blinding_factor = composer.add_input(change_blinding_factor);

        let change_value_commitment_p =
            gadgets::commitment(composer, change_value, change_blinding_factor);

        let change_value_commitment = self.change_value_commitment.into();
        composer.assert_equal_public_point(
            change_value_commitment_p,
            change_value_commitment,
        );

        // 4. Message consistency

        // 5. Prove that the value of the opening of the commitment of the
        // change Message is within range
        composer.range_gate(change_value, 64);

        // 6. Prove that the encrypted value of the opening of the commitment of
        // the Message  is within correctly encrypted to the derivative of pk
        // 7. Prove that the encrypted blinder of the opening of the commitment
        // of the Message  is within correctly encrypted to the derivative of pk
        let change_nonce = self.change_nonce.into();
        let change_nonce_p = composer.add_input(change_nonce);
        composer.constrain_to_constant(
            change_nonce_p,
            BlsScalar::zero(),
            Some(-change_nonce),
        );
        let change_nonce = change_nonce_p;

        let change_r = composer.add_input(self.change_r.into());
        let change_pk = self.change_pk.into();
        let change_pk = composer.add_public_affine(change_pk);
        let cipher_secret =
            composer.variable_base_scalar_mul(change_r, change_pk);

        let change_cipher = cipher::encrypt(
            composer,
            &cipher_secret,
            change_nonce,
            &[change_value, change_blinding_factor],
        );

        self.change_cipher
            .iter()
            .zip(change_cipher.iter())
            .for_each(|(c, w)| {
                let c = *c;

                composer.constrain_to_constant(*w, BlsScalar::zero(), Some(-c));
            });

        // 8. Prove the knowledge of the commitment opening of the commitment of
        // the output obfuscated note
        let output_value = composer.add_input(self.output_value.into());

        let output_blinding_factor = self.output_blinding_factor.into();
        let output_blinding_factor = composer.add_input(output_blinding_factor);

        let output_value_commitment_p =
            gadgets::commitment(composer, output_value, output_blinding_factor);

        let output_value_commitment = self.output_value_commitment.into();
        composer.assert_equal_public_point(
            output_value_commitment_p,
            output_value_commitment,
        );

        // 9. Prove that the value of the opening of the commitment of the
        // output obfuscated note is within range
        composer.range_gate(output_value, 64);

        // 10. Prove that v_i - v_c - v_o = 0
        composer.poly_gate(
            input_value,
            change_value,
            output_value,
            BlsScalar::zero(),
            BlsScalar::one(),
            -BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            None,
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 13
    }
}
