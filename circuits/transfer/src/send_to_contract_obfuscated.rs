// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{gadgets, DeriveKey};
use dusk_bytes::ParseHexStr;

use dusk_pki::{Ownable, SecretKey, SecretSpendKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Message};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

/// Message to be signed for the schnorr protocol.
///
/// Composed of 7 scalars and 2 ciphers.
const MESSAGE_SIZE: usize = 7 + 2 * PoseidonCipher::cipher_size();

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct StcoMessage {
    pub r: JubJubScalar,
    pub blinder: JubJubScalar,
    pub derive_key: DeriveKey,
    pub pk_r: JubJubExtended,
    pub message: Message,
}

impl StcoMessage {
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct StcoCrossover {
    pub blinder: JubJubScalar,
    crossover: Crossover,
}

impl StcoCrossover {
    pub const fn new(crossover: Crossover, blinder: JubJubScalar) -> Self {
        Self { crossover, blinder }
    }

    pub fn commitment(&self) -> &JubJubExtended {
        self.crossover.value_commitment()
    }

    pub fn nonce(&self) -> &BlsScalar {
        self.crossover.nonce()
    }

    pub fn cipher(&self) -> &[BlsScalar; PoseidonCipher::cipher_size()] {
        self.crossover.encrypted_data().cipher()
    }
}

#[derive(Debug, Clone)]
pub struct SendToContractObfuscatedCircuit {
    value: u64,
    message: StcoMessage,
    crossover: StcoCrossover,
    signature: Signature,
    address: BlsScalar,
    signature_message: BlsScalar,
    fee_pk_r: JubJubExtended,
}

impl Default for SendToContractObfuscatedCircuit {
    fn default() -> Self {
        // This signature, while still being valid, is *totally bogus*. Since
        // `Circuit` requires the `Default` trait we have to come up with a
        // "default signature"
        let signature =
            Signature::from_hex_str("40c83c7f8125fbf66ef33d30b0906eff3c23486a3cae720e16508e1fc30a110133d5d74ddf0f80803d545ae0a7cfe3156c2705aab52c27e4cdd8766bf01d218e")
                .unwrap();

        Self {
            signature,
            value: u64::default(),
            message: StcoMessage::default(),
            crossover: StcoCrossover::default(),
            address: BlsScalar::default(),
            signature_message: BlsScalar::default(),
            fee_pk_r: JubJubExtended::default(),
        }
    }
}

impl SendToContractObfuscatedCircuit {
    pub fn sign_message(
        crossover: &Crossover,
        message: &Message,
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut signature_message = [BlsScalar::zero(); MESSAGE_SIZE];
        let mut m = signature_message.iter_mut();

        crossover
            .value_commitment()
            .to_hash_inputs()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        if let Some(m) = m.next() {
            *m = *crossover.nonce();
        }

        crossover
            .encrypted_data()
            .cipher()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        message
            .value_commitment()
            .to_hash_inputs()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        if let Some(m) = m.next() {
            *m = *message.nonce();
        }

        message
            .cipher()
            .iter()
            .zip(m.by_ref())
            .for_each(|(c, m)| *m = *c);

        if let Some(m) = m.next() {
            *m = *address;
        }

        sponge::hash(&signature_message)
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        crossover_ssk: &SecretSpendKey,
        fee: &Fee,
        crossover: &Crossover,
        message: &Message,
        address: &BlsScalar,
    ) -> Signature {
        let sk_r = *crossover_ssk.sk_r(fee.stealth_address()).as_ref();
        let secret = SecretKey::from(sk_r);

        let message = Self::sign_message(crossover, message, address);

        Signature::new(&secret, rng, message)
    }

    pub fn new(
        value: u64,
        message: StcoMessage,
        crossover: StcoCrossover,
        fee: &Fee,
        address: BlsScalar,
        signature: Signature,
    ) -> Self {
        let signature_message = Self::sign_message(
            &crossover.crossover,
            message.message(),
            &address,
        );

        let fee_pk_r = *fee.stealth_address().pk_r().as_ref();

        Self {
            value,
            message,
            crossover,
            signature,
            address,
            signature_message,
            fee_pk_r,
        }
    }
}

#[allow(clippy::option_map_unit_fn)]
impl Circuit for SendToContractObfuscatedCircuit {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        let zero = C::ZERO;

        // Witnesses

        let value = composer.append_witness(self.value);

        let crossover_blinder = composer.append_witness(self.crossover.blinder);

        let message_r = composer.append_witness(self.message.r);
        let message_blinder = composer.append_witness(self.message.blinder);
        let message_derive_key_is_public =
            composer.append_witness(self.message.derive_key.is_public as u64);
        let message_derive_key_secret_a =
            composer.append_point(self.message.derive_key.secret_a);
        let message_derive_key_secret_b =
            composer.append_point(self.message.derive_key.secret_b);

        let (schnorr_u, schnorr_r) = self.signature.to_witness(composer);

        // Public inputs

        let crossover_commitment =
            composer.append_public_point(*self.crossover.commitment());
        let crossover_nonce = composer.append_public(*self.crossover.nonce());

        let mut crossover_cipher = [zero; PoseidonCipher::cipher_size()];
        self.crossover
            .cipher()
            .iter()
            .zip(crossover_cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_public(*c));

        let message_commitment =
            composer.append_public_point(*self.message.commitment());
        let message_derive_key_public_a =
            composer.append_public_point(self.message.derive_key.public_a);
        let message_derive_key_public_b =
            composer.append_public_point(self.message.derive_key.public_b);
        let message_pk_r = composer.append_public_point(self.message.pk_r);
        let message_nonce = composer.append_public(*self.message.nonce());

        let mut message_cipher = [zero; PoseidonCipher::cipher_size()];
        self.message
            .cipher()
            .iter()
            .zip(message_cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_public(*c));

        let address = composer.append_public(self.address);
        let signature_message = composer.append_public(self.signature_message);

        let fee_pk_r = composer.append_public_point(self.fee_pk_r);

        // Circuit

        // 1. commitment(Cc,Cv,Cb,64)
        gadgets::commitment(
            composer,
            crossover_commitment,
            value,
            crossover_blinder,
        )?;

        // 2. commitment(Mc,Mv,Mb,64)
        gadgets::commitment(
            composer,
            message_commitment,
            value,
            message_blinder,
        )?;

        // 3. (pa,pb) := selectPair(Mx,I,Mp,Ms)
        let message_derive_key_a = gadgets::identity_select_point(
            composer,
            message_derive_key_is_public,
            C::IDENTITY,
            message_derive_key_public_a,
            message_derive_key_secret_a,
        );

        let message_derive_key_b = gadgets::identity_select_point(
            composer,
            message_derive_key_is_public,
            C::IDENTITY,
            message_derive_key_public_b,
            message_derive_key_secret_b,
        );

        // 4. Ma == stealthAddress(Mr,(pa,pb))
        let message_stealth_address = gadgets::stealth_address(
            composer,
            message_r,
            message_derive_key_a,
            message_derive_key_b,
        )?;

        composer.assert_equal_point(message_pk_r, message_stealth_address);

        // 5. Mψ == encrypt(Mr·pa,Mn,[Mv,Mb])
        let cipher_secret =
            composer.component_mul_point(message_r, message_derive_key_a);

        gadgets::encrypt(
            composer,
            cipher_secret,
            message_nonce,
            &[value, message_blinder],
            &message_cipher,
        );

        // 6. S == H(Cc,Cn,Cψ,Mc,Mn,Mψ,A)
        let mut s = [zero; MESSAGE_SIZE];
        let mut i_s = s.iter_mut();

        i_s.next().map(|s| *s = *crossover_commitment.x());
        i_s.next().map(|s| *s = *crossover_commitment.y());
        i_s.next().map(|s| *s = crossover_nonce);

        crossover_cipher
            .iter()
            .zip(i_s.by_ref())
            .for_each(|(c, w)| *w = *c);

        i_s.next().map(|s| *s = *message_commitment.x());
        i_s.next().map(|s| *s = *message_commitment.y());
        i_s.next().map(|s| *s = message_nonce);

        message_cipher
            .iter()
            .zip(i_s.by_ref())
            .for_each(|(c, w)| *w = *c);

        i_s.next().map(|s| *s = address);

        let s = sponge::gadget(composer, &s);

        composer.assert_equal(signature_message, s);

        // 7. schnorr(σ,Fa,S)
        gadgets::schnorr_single_key_verify(
            composer, schnorr_u, schnorr_r, fee_pk_r, s,
        )?;

        // 8. Cv − Mv == 0

        Ok(())
    }
}
