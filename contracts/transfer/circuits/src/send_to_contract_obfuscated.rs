// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use anyhow::Result;
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, PublicSpendKey, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::constraint_system::ecc::scalar_mul::variable_base::variable_base_scalar_mul;
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::JubJubExtended;
use dusk_plonk::prelude::*;
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Message};
use poseidon252::cipher::{self, PoseidonCipher};
use poseidon252::sponge;
use rand_core::{CryptoRng, RngCore};
use schnorr::Signature;

#[derive(Debug, Clone)]
pub struct SendToContractObfuscatedCircuit {
    pi_positions: Vec<PublicInput>,

    signature: Signature,

    value: u64,
    blinding_factor: JubJubScalar,

    message_value: u64,
    message_blinding_factor: JubJubScalar,
    message_r: JubJubScalar,

    // Public data
    value_commitment: JubJubExtended,
    message_value_commitment: JubJubExtended,
    message_nonce: JubJubScalar,
    message_cipher: [BlsScalar; PoseidonCipher::cipher_size()],
    pk: JubJubExtended,
    message_pk: JubJubExtended,
}

impl SendToContractObfuscatedCircuit {
    pub const fn rusk_keys_id() -> &'static str {
        "transfer-send-to-contract-obfuscated"
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        fee: &Fee,
        crossover: &Crossover,
    ) -> Signature {
        let sk_r = ssk.sk_r(fee.stealth_address()).as_ref().clone();

        let secret = SecretKey::from(sk_r);
        let commitment =
            sponge::hash(&crossover.value_commitment().to_hash_inputs());

        Signature::new(&secret, rng, commitment)
    }

    pub fn new(
        crossover: &Crossover,
        fee: &Fee,
        vk: &ViewKey,
        signature: Signature,
        message: &Message,
        message_psk: &PublicSpendKey,
        message_r: JubJubScalar,
    ) -> Result<Self, PhoenixError> {
        let value_commitment = *crossover.value_commitment();
        let pk = *fee.stealth_address().pk_r().as_ref();

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

        let (message_value, message_blinding_factor) =
            message.decrypt(&message_r, message_psk)?;

        let message_pk = *message_psk.A();
        let message_value_commitment = *message.value_commitment();
        let message_nonce = *message.nonce();
        let message_cipher = *message.cipher();

        Ok(Self {
            pi_positions: vec![],
            blinding_factor,
            signature,
            value,
            value_commitment,
            pk,
            message_value,
            message_blinding_factor,
            message_r,
            message_pk,
            message_value_commitment,
            message_nonce,
            message_cipher,
        })
    }
}

impl Circuit<'_> for SendToContractObfuscatedCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let mut pi = vec![];

        // 1. Prove the knowledge of the commitment opening of the
        // commitment
        let value = composer.add_input(self.value.into());

        let blinding_factor = self.blinding_factor.into();
        let blinding_factor = composer.add_input(blinding_factor);

        let value_commitment_p =
            gadgets::commitment(composer, value, blinding_factor);

        let value_commitment = self.value_commitment.into();
        pi.push(PublicInput::AffinePoint(
            value_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        let value_commitment = value_commitment_p;

        // 2. Prove that the value of the opening of the commitment
        // of the Crossover is within range
        composer.range_gate(value, 64);

        // 3. Verify the Schnorr proof corresponding to the commitment
        // public key
        let pk = self.pk.into();
        pi.push(PublicInput::AffinePoint(
            pk,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        let pk = Point::from_public_affine(composer, pk);

        let r = Point::from_private_affine(composer, self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        gadgets::point_signature(composer, value_commitment, pk, r, u);

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

        let message_value_commitment = self.message_value_commitment.into();
        pi.push(PublicInput::AffinePoint(
            message_value_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        composer.assert_equal_public_point(
            message_value_commitment_p,
            message_value_commitment,
        );

        // 5. Prove that the value of the opening of the commitment of the
        // Message  is within range
        composer.range_gate(message_value, 64);

        // 6. Prove that the encrypted value of the opening of the commitment of
        // the Message  is within correctly encrypted to the derivative of pk
        // 7. Prove that the encrypted blinder of the opening of the commitment
        // of the Message  is within correctly encrypted to the derivative of pk
        let message_nonce = self.message_nonce.into();
        let message_nonce_p = composer.add_input(message_nonce);
        pi.push(PublicInput::BlsScalar(
            message_nonce,
            composer.circuit_size(),
        ));
        composer.constrain_to_constant(
            message_nonce_p,
            BlsScalar::zero(),
            -message_nonce,
        );
        let message_nonce = message_nonce_p;

        let message_r = composer.add_input(self.message_r.into());
        let message_pk = self.message_pk.into();
        pi.push(PublicInput::AffinePoint(
            message_pk,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));
        let message_pk = Point::from_public_affine(composer, message_pk);
        let cipher_secret =
            variable_base_scalar_mul(composer, message_r, message_pk);

        let message_cipher = cipher::encrypt(
            composer,
            cipher_secret.point(),
            message_nonce,
            &[message_value, message_blinding_factor],
        );

        self.message_cipher
            .iter()
            .zip(message_cipher.iter())
            .for_each(|(c, w)| {
                let c = *c;

                pi.push(PublicInput::BlsScalar(c, composer.circuit_size()));
                composer.constrain_to_constant(*w, BlsScalar::zero(), -c);
            });

        // 8. Prove that v_c - v_m = 0
        composer.assert_equal(value, message_value);

        self.get_mut_pi_positions().extend_from_slice(pi.as_slice());

        Ok(())
    }

    fn get_trim_size(&self) -> usize {
        1 << 14
    }

    fn set_trim_size(&mut self, _size: usize) {
        // N/A, fixed size circuit
    }

    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    /// Return a reference to the Public Inputs storage of the circuit.
    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }
}

#[test]
fn send_obfuscated() {
    use crate::test_helpers;
    use std::convert::TryInto;

    use anyhow::anyhow;
    use phoenix_core::Note;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(2322u64);

    test_helpers::circuit(
        &mut rng,
        SendToContractObfuscatedCircuit::rusk_keys_id(),
        |rng| {
            let ssk = SecretSpendKey::random(rng);
            let vk = ssk.view_key();
            let psk = ssk.public_spend_key();

            let c_value = 100;
            let c_blinding_factor = JubJubScalar::random(rng);
            let c_note =
                Note::obfuscated(rng, &psk, c_value, c_blinding_factor);
            let (fee, crossover) = c_note.try_into().map_err(|e| {
                anyhow!(
                    "Failed to convert phoenix note into crossover: {:?}",
                    e
                )
            })?;
            let c_signature = SendToContractObfuscatedCircuit::sign(
                rng, &ssk, &fee, &crossover,
            );

            let message_r = JubJubScalar::random(rng);
            let message_value = 100;
            let message = Message::new(rng, &message_r, &psk, message_value);

            SendToContractObfuscatedCircuit::new(
                &crossover,
                &fee,
                &vk,
                c_signature,
                &message,
                &psk,
                message_r,
            )
            .map_err(|e| anyhow!("Error creating circuit: {:?}", e))
        },
    )
    .expect("Failed to build and execute circuit!");
}
