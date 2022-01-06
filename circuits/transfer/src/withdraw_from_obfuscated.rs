// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{gadgets, DeriveKey};

use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::Message;

use dusk_plonk::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WfoCommitment {
    value: u64,
    blinder: JubJubScalar,
    commitment: JubJubExtended,
}

impl WfoCommitment {
    pub const fn new(
        value: u64,
        blinder: JubJubScalar,
        commitment: JubJubExtended,
    ) -> Self {
        Self {
            value,
            blinder,
            commitment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WfoChange {
    value: u64,
    message: Message,
    blinder: JubJubScalar,
    r: JubJubScalar,
    derive_key_is_public: BlsScalar,
    derive_key_secret_a: JubJubExtended,
    derive_key_secret_b: JubJubExtended,
    derive_key_public_a: JubJubExtended,
    derive_key_public_b: JubJubExtended,
    pk_r: JubJubExtended,
}

impl WfoChange {
    pub fn new(
        message: Message,
        value: u64,
        blinder: JubJubScalar,
        r: JubJubScalar,
        pk_r: JubJubExtended,
        derive_key: DeriveKey,
    ) -> Self {
        let (
            is_public,
            derive_key_secret_a,
            derive_key_secret_b,
            derive_key_public_a,
            derive_key_public_b,
        ) = derive_key.into_inner();

        let derive_key_is_public = BlsScalar::from(is_public as u64);

        Self {
            value,
            message,
            blinder,
            r,
            derive_key_is_public,
            derive_key_secret_a,
            derive_key_secret_b,
            derive_key_public_a,
            derive_key_public_b,
            pk_r,
        }
    }

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

#[derive(Debug, Clone)]
pub struct WithdrawFromObfuscatedCircuit {
    input: WfoCommitment,
    change: WfoChange,
    output: WfoCommitment,
}

impl WithdrawFromObfuscatedCircuit {
    pub fn new(
        input: WfoCommitment,
        change: WfoChange,
        output: WfoCommitment,
    ) -> Self {
        Self {
            input,
            change,
            output,
        }
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut TurboComposer,
    ) -> Result<(), PlonkError> {
        let zero = TurboComposer::constant_zero();

        // Witnesses

        let input_value = composer.append_witness(self.input.value);
        let input_blinder = composer.append_witness(self.input.blinder);

        let change_value = composer.append_witness(self.change.value);
        let change_blinder = composer.append_witness(self.change.blinder);
        let change_r = composer.append_witness(self.change.r);
        let change_derive_key_is_public =
            composer.append_witness(self.change.derive_key_is_public);
        let change_derive_key_secret_a =
            composer.append_point(self.change.derive_key_secret_a);
        let change_derive_key_secret_b =
            composer.append_point(self.change.derive_key_secret_b);

        let output_value = composer.append_witness(self.output.value);
        let output_blinder = composer.append_witness(self.output.blinder);

        // Public inputs

        let input_commitment =
            composer.append_public_point(self.input.commitment);

        let change_commitment =
            composer.append_public_point(*self.change.commitment());
        let change_derive_key_public_a =
            composer.append_public_point(self.change.derive_key_public_a);
        let change_derive_key_public_b =
            composer.append_public_point(self.change.derive_key_public_b);
        let change_pk_r = composer.append_public_point(self.change.pk_r);
        let change_nonce = composer.append_public_witness(*self.change.nonce());

        let mut change_cipher = [zero; PoseidonCipher::cipher_size()];
        self.change
            .cipher()
            .iter()
            .zip(change_cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_public_witness(*c));

        let output_commitment =
            composer.append_public_point(self.output.commitment);

        // Circuit

        // 1. commitment(Ic,Iv,Ib,64)
        gadgets::commitment(
            composer,
            input_commitment,
            input_value,
            input_blinder,
            64,
        );

        // 2. commitment(Cc,Cv,Cb,64)
        gadgets::commitment(
            composer,
            change_commitment,
            change_value,
            change_blinder,
            64,
        );

        // 3. commitment(Oc,Ov,Ob,64)
        gadgets::commitment(
            composer,
            output_commitment,
            output_value,
            output_blinder,
            64,
        );

        // 4. (pa,pb) := selectPair(Cx,I,Cp,Cs)
        let identity = composer.append_constant_identity();

        let change_derive_key_a = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            identity,
            change_derive_key_public_a,
            change_derive_key_secret_a,
        );

        let change_derive_key_b = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            identity,
            change_derive_key_public_b,
            change_derive_key_secret_b,
        );

        // 5. Ca == stealthAddress(Cr,(pa,pb))
        let change_stealth_address = gadgets::stealth_address(
            composer,
            change_r,
            change_derive_key_a,
            change_derive_key_b,
        );

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

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = Vec::with_capacity(13 + PoseidonCipher::cipher_size());

        pi.push(self.input.commitment.into());

        let commitment = *self.change.commitment();
        let nonce = *self.change.nonce();

        pi.push(commitment.into());
        pi.push(self.change.derive_key_public_a.into());
        pi.push(self.change.derive_key_public_b.into());
        pi.push(self.change.pk_r.into());
        pi.push(nonce.into());

        let cipher = self.change.cipher().iter().map(|c| (*c).into());
        pi.extend(cipher);

        pi.push(self.output.commitment.into());

        pi
    }

    fn padded_gates(&self) -> usize {
        1 << 14
    }
}
