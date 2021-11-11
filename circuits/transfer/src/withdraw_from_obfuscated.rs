// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{gadgets, CircuitDeriveKey, CircuitValueOpening};

use dusk_jubjub::JubJubExtended;
use dusk_pki::{PublicSpendKey, StealthAddress};
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::prelude::*;
use dusk_poseidon::cipher;
use phoenix_core::{Error as PhoenixError, Message};

#[derive(Debug, Clone, Copy)]
pub struct WithdrawFromObfuscatedChange {
    entropy: JubJubScalar,
    value_opening: CircuitValueOpening,
    message: Message,
    derive_key: CircuitDeriveKey,
    stealth_address: StealthAddress,
}

impl WithdrawFromObfuscatedChange {
    pub fn new(
        message: Message,
        entropy: JubJubScalar,
        psk: PublicSpendKey,
        use_public_derive_key: bool,
    ) -> Result<Self, PhoenixError> {
        let value_opening =
            CircuitValueOpening::from_message(&message, &psk, &entropy)?;

        let stealth_address = psk.gen_stealth_address(&entropy);
        let derive_key = CircuitDeriveKey::new(psk, use_public_derive_key);

        Ok(Self {
            entropy,
            value_opening,
            message,
            derive_key,
            stealth_address,
        })
    }

    pub const fn entropy(&self) -> &JubJubScalar {
        &self.entropy
    }

    pub const fn value(&self) -> u64 {
        self.value_opening.value()
    }

    pub const fn blinder(&self) -> &JubJubScalar {
        self.value_opening.blinder()
    }

    pub const fn commitment(&self) -> &JubJubExtended {
        self.value_opening.commitment()
    }

    pub const fn derive_key(&self) -> &CircuitDeriveKey {
        &self.derive_key
    }

    pub const fn stealth_address(&self) -> &StealthAddress {
        &self.stealth_address
    }

    pub fn stealth_address_pk_r(&self) -> &JubJubExtended {
        self.stealth_address.pk_r().as_ref()
    }

    pub const fn nonce(&self) -> &BlsScalar {
        self.message.nonce()
    }

    pub const fn cipher(&self) -> &[BlsScalar] {
        self.message.cipher()
    }
}

#[derive(Debug, Clone)]
pub struct WithdrawFromObfuscatedCircuit {
    input: CircuitValueOpening,
    change: WithdrawFromObfuscatedChange,
    output: CircuitValueOpening,
}

impl WithdrawFromObfuscatedCircuit {
    pub const fn new(
        input: CircuitValueOpening,
        change: WithdrawFromObfuscatedChange,
        output: CircuitValueOpening,
    ) -> Self {
        Self {
            input,
            change,
            output,
        }
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = vec![
            (*self.input.commitment()).into(),
            (*self.change.commitment()).into(),
            (*self.output.commitment()).into(),
            (*self.change.derive_key().public_a()).into(),
            (*self.change.derive_key().public_b()).into(),
            (*self.change.stealth_address_pk_r()).into(),
            (*self.change.nonce()).into(),
        ];

        pi.extend(self.change.cipher().iter().map(|c| (*c).into()));

        pi
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for WithdrawFromObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        // 1. commitment(Ic, Iv, Ib, 64)
        let iv = composer.add_input(self.input.value().into());
        let ib = composer.add_input(self.input.blinder().clone().into());
        let input_commitment = gadgets::commitment(composer, iv, ib);

        composer.range_gate(iv, 64);
        composer.assert_equal_public_point(
            input_commitment,
            self.input.commitment().clone().into(),
        );

        // 2. commitment(Cc, Cv, Cb, 64)
        let cv = composer.add_input(self.change.value().into());
        let cb = composer.add_input(self.change.blinder().clone().into());
        let change_commitment = gadgets::commitment(composer, cv, cb);

        composer.range_gate(cv, 64);
        composer.assert_equal_public_point(
            change_commitment,
            self.change.commitment().clone().into(),
        );

        // 3. commitment(Oc, Ov, Ob, 64)
        let ov = composer.add_input(self.output.value().into());
        let ob = composer.add_input(self.output.blinder().clone().into());
        let output_commitment = gadgets::commitment(composer, ov, ob);

        composer.range_gate(ov, 64);
        composer.assert_equal_public_point(
            output_commitment,
            self.output.commitment().clone().into(),
        );

        // 4. (pa, pb) := selectPair(Cx, I, Cp, Cs)
        let identity = Point::identity(composer);
        let change_derive_key_is_public =
            composer.add_input(self.change.derive_key().is_public().clone());

        let change_derive_key_public_a = composer.add_public_affine(
            self.change.derive_key().public_a().clone().into(),
        );
        let change_derive_key_secret_a = composer
            .add_affine(self.change.derive_key().secret_a().clone().into());

        let change_derive_key_a = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            identity,
            change_derive_key_public_a,
            change_derive_key_secret_a,
        );

        let change_derive_key_public_b = composer.add_public_affine(
            self.change.derive_key().public_b().clone().into(),
        );
        let change_derive_key_secret_b = composer
            .add_affine(self.change.derive_key().secret_b().clone().into());

        let change_derive_key_b = gadgets::identity_select_point(
            composer,
            change_derive_key_is_public,
            identity,
            change_derive_key_public_b,
            change_derive_key_secret_b,
        );

        // 5. Ca == stealthAddress(Cr, (pa, pb))
        let change_entropy =
            composer.add_input(self.change.entropy().clone().into());
        let change_stealth_address = gadgets::stealth_address(
            composer,
            change_entropy,
            change_derive_key_a,
            change_derive_key_b,
        );

        composer.assert_equal_public_point(
            change_stealth_address,
            self.change.stealth_address_pk_r().clone().into(),
        );

        // 6. Cψ == encrypt(Cr · pa, Cn, [Cv, Cb])
        let change_cipher_secret = composer
            .variable_base_scalar_mul(change_entropy, change_derive_key_a);

        let change_nonce =
            composer.add_input(self.change.nonce().clone().into());
        composer.constrain_to_constant(
            change_nonce,
            BlsScalar::zero(),
            Some(-self.change.nonce()),
        );

        let change_cipher = cipher::encrypt(
            composer,
            &change_cipher_secret,
            change_nonce,
            &[cv, cb],
        );

        self.change
            .cipher()
            .iter()
            .zip(change_cipher.iter())
            .for_each(|(c, w)| {
                let c = *c;

                composer.constrain_to_constant(*w, BlsScalar::zero(), Some(-c));
            });

        // 7. Iv − Cv − Ov == 0
        composer.poly_gate(
            iv,
            cv,
            ov,
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
        1 << 14
    }
}
