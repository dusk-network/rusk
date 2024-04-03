// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_plonk::prelude::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use jubjub_schnorr::Signature;
use phoenix_core::{Crossover, Fee, Ownable, SecretKey};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

/// Message to be signed for the schnorr protocol.
///
/// Composed of 5 scalars and 1 cipher.
const MESSAGE_SIZE: usize = 5 + PoseidonCipher::cipher_size();

#[derive(Debug, Clone)]
pub struct SendToContractTransparentCircuit {
    value: BlsScalar,
    blinder: JubJubScalar,
    commitment: JubJubExtended,
    nonce: BlsScalar,
    cipher: [BlsScalar; PoseidonCipher::cipher_size()],
    note_pk: JubJubExtended,
    address: BlsScalar,
    message: BlsScalar,
    signature: Signature,
}

impl Default for SendToContractTransparentCircuit {
    fn default() -> Self {
        // This signature, while still being valid, is *totally bogus*. Since
        // `Circuit` requires the `Default` trait we have to come up with a
        // "default signature"
        use dusk_bytes::ParseHexStr;
        let signature =
            Signature::from_hex_str("40c83c7f8125fbf66ef33d30b0906eff3c23486a3cae720e16508e1fc30a110133d5d74ddf0f80803d545ae0a7cfe3156c2705aab52c27e4cdd8766bf01d218e")
        .unwrap();

        Self {
            signature,
            value: BlsScalar::default(),
            blinder: JubJubScalar::default(),
            commitment: JubJubExtended::default(),
            nonce: BlsScalar::default(),
            cipher: [BlsScalar::default(); PoseidonCipher::cipher_size()],
            note_pk: JubJubExtended::default(),
            address: BlsScalar::default(),
            message: BlsScalar::default(),
        }
    }
}

impl SendToContractTransparentCircuit {
    pub fn sign_message(
        crossover: &Crossover,
        value: u64,
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut message = [BlsScalar::zero(); MESSAGE_SIZE];
        let mut m = message.iter_mut();

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

        if let Some(m) = m.next() {
            *m = value.into();
        }
        if let Some(m) = m.next() {
            *m = *address;
        }

        sponge::hash(&message)
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        sk: &SecretKey,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        address: &BlsScalar,
    ) -> Signature {
        let note_sk = sk.sk_r(fee.stealth_address());
        let message = Self::sign_message(crossover, value, address);

        note_sk.sign(rng, message)
    }

    pub fn new(
        fee: &Fee,
        crossover: &Crossover,
        crossover_value: u64,
        crossover_blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Self {
        let value = crossover_value;
        let blinder = crossover_blinder;
        let commitment = *crossover.value_commitment();
        let nonce = *crossover.nonce();
        let cipher = *crossover.encrypted_data().cipher();

        let message = Self::sign_message(crossover, value, &address);
        let value = value.into();

        let note_pk = *fee.stealth_address().pk_r().as_ref();

        Self {
            value,
            blinder,
            commitment,
            nonce,
            cipher,
            note_pk,
            address,
            message,
            signature,
        }
    }
}

#[allow(clippy::option_map_unit_fn)]
impl Circuit for SendToContractTransparentCircuit {
    fn circuit(&self, composer: &mut Composer) -> Result<(), PlonkError> {
        // Witnesses

        let blinder = composer.append_witness(self.blinder);
        let nonce = composer.append_witness(self.nonce);

        let mut cipher = [Composer::ZERO; PoseidonCipher::cipher_size()];
        self.cipher
            .iter()
            .zip(cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_witness(*c));

        let (schnorr_u, schnorr_r) = self.signature.append(composer);
        let address = composer.append_witness(self.address);

        // Public inputs

        let commitment = composer.append_public_point(self.commitment);
        let value = composer.append_public(self.value);
        let note_pk = composer.append_public_point(self.note_pk);
        let _ = composer.append_public(self.message);

        // 1. commitment(Cc,Cv,Cb,64)
        gadgets::commitment(composer, commitment, value, blinder)?;

        // 2. S == H(Cc,Cn,Cψ,Cv,A)
        let mut s = [Composer::ZERO; MESSAGE_SIZE];
        let mut i_s = s.iter_mut();

        i_s.next().map(|s| *s = *commitment.x());
        i_s.next().map(|s| *s = *commitment.y());
        i_s.next().map(|s| *s = nonce);

        cipher.iter().zip(i_s.by_ref()).for_each(|(c, w)| *w = *c);

        i_s.next().map(|s| *s = value);
        i_s.next().map(|s| *s = address);

        let s = sponge::gadget(composer, &s);

        // 3. schnorr(σ,Fa,S)
        gadgets::schnorr_verify_signature(
            composer, schnorr_u, schnorr_r, note_pk, s,
        )?;

        Ok(())
    }
}
