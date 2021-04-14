// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use anyhow::Result;
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::JubJubAffine;
use dusk_plonk::prelude::Error as PlonkError;
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use phoenix_core::{Crossover, Error as PhoenixError, Fee};
use rand_core::{CryptoRng, RngCore};
use schnorr::Signature;

#[derive(Debug, Clone)]
pub struct SendToContractTransparentCircuit {
    crossover: Crossover,
    crossover_blinder: JubJubScalar,
    fee: Fee,
    message: BlsScalar,
    signature: Signature,
    value: BlsScalar,
}

impl SendToContractTransparentCircuit {
    pub const fn rusk_keys_id() -> &'static str {
        "transfer-send-to-contract-transparent"
    }

    pub fn sign_message(
        crossover: &Crossover,
        value: u64,
        address: &BlsScalar,
    ) -> BlsScalar {
        let mut message = crossover.to_hash_inputs().to_vec();

        message.push(value.into());
        message.push(*address);

        sponge::hash(message.as_slice())
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        address: &BlsScalar,
    ) -> Signature {
        let sk_r = ssk.sk_r(fee.stealth_address()).as_ref().clone();
        let secret = SecretKey::from(sk_r);

        let message = Self::sign_message(crossover, value, address);

        Signature::new(&secret, rng, message)
    }

    pub fn new(
        fee: Fee,
        crossover: Crossover,
        vk: &ViewKey,
        address: &BlsScalar,
        signature: Signature,
    ) -> Result<Self, PhoenixError> {
        let nonce = BlsScalar::from(*crossover.nonce());
        let secret = fee.stealth_address().R() * vk.a();
        let (value, crossover_blinder) = crossover
            .encrypted_data()
            .decrypt(&secret.into(), &nonce)
            .map(|d| {
                let value = d[0].reduce().0[0];
                let crossover_blinder =
                    JubJubScalar::from_bytes(&d[1].to_bytes())
                        .unwrap_or_default();

                (value, crossover_blinder)
            })?;

        let message = Self::sign_message(&crossover, value, address);
        let value = BlsScalar::from(value);

        Ok(Self {
            crossover,
            crossover_blinder,
            fee,
            message,
            signature,
            value,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        // step 1
        let value_commitment = self.crossover.value_commitment();
        let value_commitment = JubJubAffine::from(value_commitment);

        // step 3
        let pk = self.fee.stealth_address().pk_r().as_ref();
        let pk = JubJubAffine::from(pk);

        let message = self.message.into();

        // step 4
        let value = self.value.into();

        vec![value_commitment.into(), pk.into(), message, value]
    }
}

impl Circuit for SendToContractTransparentCircuit {
    // TODO Define ID
    const CIRCUIT_ID: [u8; 32] = [0xff; 32];

    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        // 1. Prove the knowledge of the commitment opening of the
        // commitment
        let value = composer.add_input(self.value);

        let crossover_blinder = self.crossover_blinder.into();
        let crossover_blinder = composer.add_input(crossover_blinder);

        let value_commitment_p =
            gadgets::commitment(composer, value, crossover_blinder);

        let value_commitment = self.crossover.value_commitment();
        let value_commitment = JubJubAffine::from(value_commitment);
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        // 2. Prove that the value of the opening of the commitment
        // of the Crossover is within range
        composer.range_gate(value, 64);

        // 3. Verify the Schnorr proof corresponding to the commitment
        // public key
        let pk = self.fee.stealth_address().pk_r().as_ref().into();
        let pk = Point::from_public_affine(composer, pk);

        let r = Point::from_private_affine(composer, self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        let message = composer.add_input(self.message);
        composer.constrain_to_constant(
            message,
            BlsScalar::zero(),
            Some(-self.message),
        );

        schnorr::gadgets::single_key_verify(composer, r, u, pk, message);

        // 4. Prove that v_c - v = 0
        composer.constrain_to_constant(
            value,
            BlsScalar::zero(),
            Some(-self.value),
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 13
    }
}
