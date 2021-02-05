// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{circuit_common_methods, gadgets, rusk_profile_methods};

use anyhow::{anyhow, Result};
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::JubJubExtended;
use dusk_plonk::prelude::*;
use phoenix_core::{Crossover, Error as PhoenixError, Fee};
use poseidon252::sponge;
use rand_core::{CryptoRng, RngCore};
use schnorr::Signature;

#[derive(Debug, Clone)]
pub struct SendToContractTransparentCircuit {
    pi_positions: Vec<PublicInput>,

    blinding_factor: JubJubScalar,
    signature: Signature,

    // Public data
    value_commitment: JubJubExtended,
    pk: JubJubExtended,
    value: BlsScalar,
}

impl SendToContractTransparentCircuit {
    rusk_profile_methods!(self, {
        "transfer-send-to-contract-transparent".into()
    });

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
            pi_positions: vec![],
            blinding_factor,
            signature,
            value: BlsScalar::from(value),
            value_commitment,
            pk,
        })
    }
}

impl Circuit<'_> for SendToContractTransparentCircuit {
    circuit_common_methods!(13);

    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let mut pi = vec![];

        // 1. Prove the knowledge of the commitment opening of the
        // commitment
        let value = composer.add_input(self.value);

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
        gadgets::range(composer, value);

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

        // 4. Prove that v_c - v = 0
        pi.push(PublicInput::BlsScalar(self.value, composer.circuit_size()));
        composer.constrain_to_constant(value, BlsScalar::zero(), -self.value);

        self.get_mut_pi_positions().extend_from_slice(pi.as_slice());

        Ok(())
    }
}

#[cfg(test)]
crate::test_circuit!(send_transparent, {
    use std::convert::TryInto;

    use phoenix_core::Note;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(2322u64);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_vk = c_ssk.view_key();
    let c_psk = c_ssk.public_spend_key();

    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);

    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (fee, crossover) = c_note.try_into().map_err(|e| {
        anyhow!("Failed to convert phoenix note into crossover: {:?}", e)
    })?;

    let c_signature = SendToContractTransparentCircuit::sign(
        &mut rng, &c_ssk, &fee, &crossover,
    );

    SendToContractTransparentCircuit::new(&fee, &crossover, &c_vk, c_signature)
        .unwrap()
});
