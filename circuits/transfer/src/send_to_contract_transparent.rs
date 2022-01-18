// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_pki::{Ownable, SecretKey, SecretSpendKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Fee};
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
    pk_r: JubJubExtended,
    address: BlsScalar,
    message: BlsScalar,
    signature: Signature,
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
        ssk: &SecretSpendKey,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        address: &BlsScalar,
    ) -> Signature {
        let sk_r = *ssk.sk_r(fee.stealth_address()).as_ref();
        let secret = SecretKey::from(sk_r);

        let message = Self::sign_message(crossover, value, address);

        Signature::new(&secret, rng, message)
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

        let pk_r = *fee.stealth_address().pk_r().as_ref();

        Self {
            value,
            blinder,
            commitment,
            nonce,
            cipher,
            pk_r,
            address,
            message,
            signature,
        }
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for SendToContractTransparentCircuit {
    fn gadget(
        &mut self,
        composer: &mut TurboComposer,
    ) -> Result<(), PlonkError> {
        let zero = TurboComposer::constant_zero();

        // Witnesses

        let blinder = composer.append_witness(self.blinder);
        let nonce = composer.append_witness(self.nonce);

        let mut cipher = [zero; PoseidonCipher::cipher_size()];
        self.cipher
            .iter()
            .zip(cipher.iter_mut())
            .for_each(|(c, w)| *w = composer.append_witness(*c));

        let (schnorr_u, schnorr_r) = self.signature.to_witness(composer);
        let address = composer.append_witness(self.address);

        // Public inputs

        let commitment = composer.append_public_point(self.commitment);
        let value = composer.append_public_witness(self.value);
        let pk_r = composer.append_public_point(self.pk_r);
        let message = composer.append_public_witness(self.message);

        // 1. commitment(Cc,Cv,Cb,64)
        gadgets::commitment(composer, commitment, value, blinder, 64);

        // 2. S == H(Cc,Cn,Cψ,Cv,A)
        let mut s = [zero; MESSAGE_SIZE];
        let mut i_s = s.iter_mut();

        i_s.next().map(|s| *s = *commitment.x());
        i_s.next().map(|s| *s = *commitment.y());
        i_s.next().map(|s| *s = nonce);

        cipher.iter().zip(i_s.by_ref()).for_each(|(c, w)| *w = *c);

        i_s.next().map(|s| *s = value);
        i_s.next().map(|s| *s = address);

        let s = sponge::gadget(composer, &s);

        // 3. schnorr(σ,Fa,S)
        gadgets::schnorr_single_key_verify(
            composer, schnorr_u, schnorr_r, pk_r, s,
        );

        Ok(())
    }

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        // 1. commitment(Cc,Cv,Cb,64)
        let commitment = self.commitment.into();
        let value = self.value.into();

        // 3. 3. schnorr(σ,Fa,S)
        let pk_r = self.pk_r.into();
        let message = self.message.into();

        vec![commitment, value, pk_r, message]
    }

    fn padded_gates(&self) -> usize {
        1 << 13
    }
}
