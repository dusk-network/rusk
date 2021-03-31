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
use dusk_plonk::jubjub::{JubJubAffine, JubJubExtended};
use dusk_plonk::prelude::Error as PlonkError;
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use phoenix_core::{Crossover, Error as PhoenixError, Fee};
use rand_core::{CryptoRng, RngCore};
use schnorr::Signature;

#[derive(Debug, Clone)]
pub struct SendToContractTransparentCircuit {
    blinding_factor: JubJubScalar,
    signature: Signature,

    // Public data
    value_commitment: JubJubExtended,
    pk: JubJubExtended,
    value: BlsScalar,
}

impl SendToContractTransparentCircuit {
    pub const fn rusk_keys_id() -> &'static str {
        "transfer-send-to-contract-transparent"
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
        fee: &Fee,
        crossover: &Crossover,
        vk: &ViewKey,
        signature: Signature,
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

        Ok(Self {
            blinding_factor,
            signature,
            value: BlsScalar::from(value),
            value_commitment,
            pk,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        // step 1
        let value_commitment = JubJubAffine::from(self.value_commitment);

        // step 3
        let pk = JubJubAffine::from(self.pk);

        // step 4
        let value = self.value.into();

        vec![value_commitment.into(), pk.into(), value]
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

        let blinding_factor = self.blinding_factor.into();
        let blinding_factor = composer.add_input(blinding_factor);

        let value_commitment_p =
            gadgets::commitment(composer, value, blinding_factor);

        let value_commitment = self.value_commitment.into();
        composer
            .assert_equal_public_point(value_commitment_p, value_commitment);

        let value_commitment = value_commitment_p;

        // 2. Prove that the value of the opening of the commitment
        // of the Crossover is within range
        composer.range_gate(value, 64);

        // 3. Verify the Schnorr proof corresponding to the commitment
        // public key
        let pk = self.pk.into();
        let pk = Point::from_public_affine(composer, pk);

        let r = Point::from_private_affine(composer, self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        gadgets::point_signature(composer, value_commitment, pk, r, u);

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
