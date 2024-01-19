// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{gadgets, DeriveKey};

use dusk_plonk::prelude::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::Message;

use dusk_plonk::prelude::*;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WfoCommitment {
    pub value: u64,
    pub blinder: JubJubScalar,
    pub commitment: JubJubExtended,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WfoChange {
    pub value: u64,
    pub message: Message,
    pub blinder: JubJubScalar,
    pub r: JubJubScalar,
    pub derive_key: DeriveKey,
    pub pk_r: JubJubExtended,
}

impl WfoChange {
    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn commitment(&self) -> &JubJubExtended {
        self.message.value_commitment()
    }

    pub fn nonce(&self) -> &BlsScalar {
        self.message.nonce()
    }

    pub fn cipher(&self) -> &[BlsScalar; PoseidonCipher::cipher_size()] {
        self.message.cipher()
    }
}

#[derive(Debug, Default, Clone)]
pub struct WithdrawFromObfuscatedCircuit {
    pub input: WfoCommitment,
    pub change: WfoChange,
    pub output: WfoCommitment,
}

impl Circuit for WithdrawFromObfuscatedCircuit {
    fn circuit(&self, composer: &mut Composer) -> Result<(), PlonkError> {
        let zero = Composer::ZERO;

        // Witnesses

        let input_value = composer.append_witness(self.input.value);
        let input_blinder = composer.append_witness(self.input.blinder);

        let change_value = composer.append_witness(self.change.value);
        let change_blinder = composer.append_witness(self.change.blinder);
        let change_r = composer.append_witness(self.change.r);
        let change_derive_key_is_public =
            composer.append_witness(self.change.derive_key.is_public as u64);
        let change_derive_key_secret_a =
            composer.append_point(self.change.derive_key.secret_a);
        let change_derive_key_secret_b =
            composer.append_point(self.change.derive_key.secret_b);

        let output_value = composer.append_witness(self.output.value);
        let output_blinder = composer.append_witness(self.output.blinder);

        // Public inputs

        let input_commitment =
            composer.append_public_point(self.input.commitment);

        let change_commitment =
            composer.append_public_point(*self.change.commitment());
        let change_derive_key_public_a =
            composer.append_public_point(self.change.derive_key.public_a);
        let change_derive_key_public_b =
            composer.append_public_point(self.change.derive_key.public_b);
        let change_pk_r = composer.append_public_point(self.change.pk_r);
        let change_nonce = composer.append_public(*self.change.nonce());

        let mut change_cipher = [zero; PoseidonCipher::cipher_size()];
        self.change
            .cipher()
            .iter()
            .zip(change_cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_public(*c));

        let output_commitment =
            composer.append_public_point(self.output.commitment);

        // Circuit

        // 1. commitment(Ic,Iv,Ib,64)
        gadgets::commitment(
            composer,
            input_commitment,
            input_value,
            input_blinder,
        )?;

        // 2. commitment(Cc,Cv,Cb,64)
        gadgets::commitment(
            composer,
            change_commitment,
            change_value,
            change_blinder,
        )?;

        // 3. commitment(Oc,Ov,Ob,64)
        gadgets::commitment(
            composer,
            output_commitment,
            output_value,
            output_blinder,
        )?;

        // 4. (pa,pb) := selectPair(Cx,I,Cp,Cs)
        let change_derive_key_a = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            Composer::IDENTITY,
            change_derive_key_public_a,
            change_derive_key_secret_a,
        );

        let change_derive_key_b = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            Composer::IDENTITY,
            change_derive_key_public_b,
            change_derive_key_secret_b,
        );

        // 5. Ca == stealthAddress(Cr,(pa,pb))
        let change_stealth_address = gadgets::stealth_address(
            composer,
            change_r,
            change_derive_key_a,
            change_derive_key_b,
        )?;

        composer.assert_equal_point(change_pk_r, change_stealth_address);

        // 6. Cψ == encrypt(Cr·pa,Cn,[Cv,Cb])
        let cipher_secret =
            composer.component_mul_point(change_r, change_derive_key_a);

        gadgets::encrypt(
            composer,
            cipher_secret,
            change_nonce,
            &[change_value, change_blinder],
            &change_cipher,
        );

        // 7. Iv − Cv − Ov == 0
        let constraint = Constraint::new()
            .left(1)
            .a(input_value)
            .right(-BlsScalar::one())
            .b(change_value)
            .fourth(-BlsScalar::one())
            .d(output_value);

        composer.append_gate(constraint);

        Ok(())
    }
}
