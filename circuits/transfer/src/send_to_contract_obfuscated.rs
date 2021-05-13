// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use anyhow::Result;
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, PublicSpendKey, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended};
use dusk_plonk::prelude::*;
use dusk_poseidon::{cipher, sponge};
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Message};
use rand_core::{CryptoRng, RngCore};

#[derive(Debug, Clone)]
pub struct SendToContractObfuscatedCircuit {
    fee: Fee,
    crossover: Crossover,
    signature: Signature,
    hash: BlsScalar,
    public_message_pk: BlsScalar,
    message: Message,
    message_value: u64,
    message_blinding_factor: JubJubScalar,
    message_r: JubJubScalar,
    message_pk: JubJubExtended,
    message_private_pk: JubJubExtended,
    value: u64,
    blinding_factor: JubJubScalar,
    address: BlsScalar,
}

impl SendToContractObfuscatedCircuit {
    pub const fn rusk_keys_id() -> &'static str {
        "transfer-send-to-contract-obfuscated"
    }

    pub fn sign_message(
        crossover: &Crossover,
        message: &Message,
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut inputs = crossover.to_hash_inputs().to_vec();

        inputs.extend(&message.to_hash_inputs());
        inputs.push(*address);

        sponge::hash(inputs.as_slice())
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        fee: &Fee,
        crossover: &Crossover,
        message: &Message,
        address: &BlsScalar,
    ) -> Signature {
        let sk_r = ssk.sk_r(fee.stealth_address()).as_ref().clone();
        let secret = SecretKey::from(sk_r);

        let message = Self::sign_message(crossover, message, address);

        Signature::new(&secret, rng, message)
    }

    pub fn new(
        fee: Fee,
        crossover: Crossover,
        vk: &ViewKey,
        signature: Signature,
        public_message_pk: bool,
        message: Message,
        message_psk: &PublicSpendKey,
        message_r: JubJubScalar,
        address: BlsScalar,
    ) -> Result<Self, PhoenixError> {
        let nonce = BlsScalar::from(*crossover.nonce());
        let secret = fee.stealth_address().R() * vk.a();
        let (value, blinding_factor) = crossover
            .encrypted_data()
            .decrypt(&secret.into(), &nonce)
            .map(|d| {
                let value = d[0].reduce().0[0];
                let blinding_factor =
                    JubJubScalar::from_bytes(&d[1].to_bytes())
                        .unwrap_or_default();

                (value, blinding_factor)
            })?;

        let hash = Self::sign_message(&crossover, &message, &address);

        let (message_value, message_blinding_factor) =
            message.decrypt(&message_r, message_psk)?;
        let message_pk = *message_psk.A();
        let (message_pk, message_private_pk) = if public_message_pk {
            (message_pk, JubJubAffine::identity().into())
        } else {
            (JubJubAffine::identity().into(), message_pk)
        };

        let public_message_pk = BlsScalar::from(public_message_pk as u64);

        Ok(Self {
            fee,
            crossover,
            message,
            signature,
            hash,
            public_message_pk,
            message_value,
            message_blinding_factor,
            message_r,
            message_pk,
            message_private_pk,
            value,
            blinding_factor,
            address,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = vec![];

        // step 1
        let value_commitment = self.crossover.value_commitment();
        let value_commitment = JubJubAffine::from(value_commitment);
        pi.push(value_commitment.into());

        // step 4
        let message_value_commitment = self.message.value_commitment();
        let message_value_commitment =
            JubJubAffine::from(message_value_commitment);
        pi.push(message_value_commitment.into());

        // step 7
        pi.push(self.message.nonce().clone().into());

        let message_pk = JubJubAffine::from(self.message_pk);
        pi.push(message_pk.into());

        let identity = JubJubAffine::identity();
        pi.push(identity.into());

        pi.extend(self.message.cipher().iter().map(|c| (*c).into()));

        // step 3
        let pk = self.fee.stealth_address().pk_r().as_ref();
        let pk = JubJubAffine::from(pk);
        pi.push(pk.into());

        let hash = self.hash.into();
        pi.push(hash);

        pi
    }

    /// Check if the internal private message key is set to identity
    pub fn is_private_message_pk_identity(&self) -> bool {
        self.message_private_pk == JubJubAffine::identity().into()
    }

    /// Check if the internal public message key is set to identity
    pub fn is_public_message_pk_identity(&self) -> bool {
        self.message_pk == JubJubAffine::identity().into()
    }
}

impl Circuit for SendToContractObfuscatedCircuit {
    // TODO Define ID
    const CIRCUIT_ID: [u8; 32] = [0xff; 32];

    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        // 1. Prove the knowledge of the commitment opening of the
        // commitment
        let value = composer.add_input(self.value.into());

        let blinding_factor = self.blinding_factor.into();
        let blinding_factor = composer.add_input(blinding_factor);

        let value_commitment_p =
            gadgets::commitment(composer, value, blinding_factor);

        let value_commitment = self.crossover.value_commitment().into();
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        let value_commitment = value_commitment_p;

        // 2. Prove that the value of the opening of the commitment
        // of the Crossover is within range
        composer.range_gate(value, 64);

        // 4. Prove the knowledge of the commitment opening of the commitment of
        // the message
        let message_value = composer.add_input(self.message_value.into());
        let message_blinding_factor =
            composer.add_input(self.message_blinding_factor.into());

        let message_value_commitment_p = gadgets::commitment(
            composer,
            message_value,
            message_blinding_factor,
        );

        let message_value_commitment = self.message.value_commitment().into();
        composer.assert_equal_public_point(
            message_value_commitment_p,
            message_value_commitment,
        );

        let message_value_commitment = message_value_commitment_p;

        // 5. Prove that the value of the opening of the commitment of the
        // Message  is within range
        composer.range_gate(message_value, 64);

        // 6. Prove that the encrypted value of the opening of the commitment of
        // the Message  is within correctly encrypted to the derivative of pk
        // 7. Prove that the encrypted blinder of the opening of the commitment
        // of the Message  is within correctly encrypted to the derivative of pk

        // Prove `public_message_pk is [0, 1]`
        let public_message_pk = composer.add_input(self.public_message_pk);
        composer.poly_gate(
            public_message_pk,
            public_message_pk,
            public_message_pk,
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            BlsScalar::zero(),
            None,
        );

        let message_nonce = self.message.nonce().clone().into();
        let message_nonce_p = composer.add_input(message_nonce);
        composer.constrain_to_constant(
            message_nonce_p,
            BlsScalar::zero(),
            Some(-message_nonce),
        );
        let message_nonce = message_nonce_p;

        let message_r = composer.add_input(self.message_r.into());

        let message_pk = self.message_pk.into();
        let message_pk = composer.add_public_affine(message_pk);

        let message_private_pk = self.message_private_pk.into();
        let message_private_pk = composer.add_affine(message_private_pk);

        let message_pk_identity = composer.conditional_point_select(
            message_private_pk,
            message_pk,
            public_message_pk,
        );
        composer.assert_equal_public_point(
            message_pk_identity,
            JubJubAffine::identity(),
        );

        let message_pk =
            composer.point_addition_gate(message_pk, message_private_pk);
        let cipher_secret =
            composer.variable_base_scalar_mul(message_r, message_pk);

        let message_cipher = cipher::encrypt(
            composer,
            &cipher_secret,
            message_nonce,
            &[message_value, message_blinding_factor],
        );

        self.message
            .cipher()
            .iter()
            .zip(message_cipher.iter())
            .for_each(|(c, w)| {
                let c = *c;

                composer.constrain_to_constant(*w, BlsScalar::zero(), Some(-c));
            });

        // 3. Verify the Schnorr proof corresponding to the commitment
        // public key
        let pk = self.fee.stealth_address().pk_r().as_ref().into();
        let pk = composer.add_public_affine(pk);

        let r = composer.add_affine(self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        let nonce = composer.add_input(self.crossover.nonce().clone().into());
        let address = composer.add_input(self.address);

        let mut inputs =
            vec![*value_commitment.x(), *value_commitment.y(), nonce];

        let encrypted_data = self
            .crossover
            .encrypted_data()
            .cipher()
            .iter()
            .map(|d| composer.add_input(*d));
        inputs.extend(encrypted_data);

        inputs.push(*message_value_commitment.x());
        inputs.push(*message_value_commitment.y());
        inputs.push(message_nonce);
        inputs.extend(message_cipher.iter());
        inputs.push(address);

        let hash = sponge::gadget(composer, inputs.as_slice());
        composer.constrain_to_constant(
            hash,
            BlsScalar::zero(),
            Some(-self.hash),
        );

        dusk_schnorr::gadgets::single_key_verify(composer, r, u, pk, hash);

        // 8. Prove that v_c - v_m = 0
        composer.assert_equal(value, message_value);

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 14
    }
}
