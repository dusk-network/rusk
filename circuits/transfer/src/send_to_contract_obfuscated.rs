// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubAffine, JubJubExtended};
use dusk_pki::{Ownable, PublicSpendKey, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::prelude::*;
use dusk_poseidon::{cipher, sponge};
use dusk_schnorr::Signature;
use phoenix_core::{Crossover, Error as PhoenixError, Fee, Message};
use rand_core::{CryptoRng, RngCore};

#[derive(Debug, Clone)]
pub struct SendToContractObfuscatedCircuit {
    // Private
    crossover_value: u64,
    crossover_blinding_factor: JubJubScalar,
    message_value: u64,
    message_blinding_factor: JubJubScalar,
    message_derive_key_is_public: BlsScalar,
    message_derive_key_private_a: JubJubExtended,
    message_derive_key_private_b: JubJubExtended,
    message_r: JubJubScalar,
    signature: Signature,

    // Public
    crossover: Crossover,
    message: Message,
    message_derive_key_public_a: JubJubExtended,
    message_derive_key_public_b: JubJubExtended,
    message_pk_r: JubJubExtended,
    address: BlsScalar,
    hash: BlsScalar,
    fee: Fee,
}

impl SendToContractObfuscatedCircuit {
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
        message_derive_key_is_public: bool,
        message: Message,
        message_psk: &PublicSpendKey,
        message_r: JubJubScalar,
        address: BlsScalar,
    ) -> Result<Self, PhoenixError> {
        let nonce = BlsScalar::from(*crossover.nonce());
        let secret = fee.stealth_address().R() * vk.a();
        let (crossover_value, crossover_blinding_factor) = crossover
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

        let message_pk_r =
            *message_psk.gen_stealth_address(&message_r).pk_r().as_ref();

        let (message_value, message_blinding_factor) =
            message.decrypt(&message_r, message_psk)?;

        let identity = JubJubAffine::identity().into();
        let message_derive_key_a = *message_psk.A();
        let message_derive_key_b = *message_psk.B();

        let (message_derive_key_public_a, message_derive_key_private_a) =
            if message_derive_key_is_public {
                (message_derive_key_a, identity)
            } else {
                (identity, message_derive_key_a)
            };

        let (message_derive_key_public_b, message_derive_key_private_b) =
            if message_derive_key_is_public {
                (message_derive_key_b, identity)
            } else {
                (identity, message_derive_key_b)
            };

        let message_derive_key_is_public =
            BlsScalar::from(message_derive_key_is_public as u64);

        Ok(Self {
            fee,
            crossover,
            message,
            signature,
            hash,
            message_value,
            message_blinding_factor,
            message_r,
            message_pk_r,
            message_derive_key_is_public,
            message_derive_key_public_a,
            message_derive_key_private_a,
            message_derive_key_public_b,
            message_derive_key_private_b,
            crossover_value,
            crossover_blinding_factor,
            address,
        })
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = vec![];

        //  1. commitment(Cc,vc,bc)
        let value_commitment = self.crossover.value_commitment();
        let value_commitment = JubJubAffine::from(value_commitment);
        pi.push(value_commitment.into());

        //  3. commitment(Cm,vm,bm)
        let message_value_commitment = self.message.value_commitment();
        let message_value_commitment =
            JubJubAffine::from(message_value_commitment);
        pi.push(message_value_commitment.into());

        //  5. Da:=select(fm,I,Dap,Das)
        let message_derive_key_public_a =
            JubJubAffine::from(self.message_derive_key_public_a);
        pi.push(message_derive_key_public_a.into());

        //  6. Db:=select(fm,I,Dbp,Dbs)
        let message_derive_key_public_b =
            JubJubAffine::from(self.message_derive_key_public_b);
        pi.push(message_derive_key_public_b.into());

        //  7. Km:=stealthAddress(mr,Da,Db)
        let message_pk_r = JubJubAffine::from(self.message_pk_r);
        pi.push(message_pk_r.into());

        //  9. Em==encrypt(es,Nm,[vm,bm])
        pi.push(self.message.nonce().clone().into());
        pi.extend(self.message.cipher().iter().map(|c| (*c).into()));

        // 10. Hs==H([Cc,Nc,Ec,Cm,Nm,Em,Ac])
        pi.push(self.address.clone().into());
        pi.push(self.crossover.nonce().clone().into());
        pi.extend(
            self.crossover
                .encrypted_data()
                .cipher()
                .iter()
                .map(|c| (*c).into()),
        );
        pi.push(self.hash.into());

        // 11. schnorrVerify(σ,Kf,H)
        let fee_pk_r = self.fee.stealth_address().pk_r().as_ref();
        let fee_pk_r = JubJubAffine::from(fee_pk_r);
        pi.push(fee_pk_r.into());

        pi
    }

    /// Check if the message derive key is stored as public for this circuit
    pub fn is_message_derive_key_public(&self) -> bool {
        self.message_derive_key_private_a == JubJubAffine::identity().into()
    }
}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for SendToContractObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        //  1. commitment(Cc,vc,bc)
        let crossover_value = composer.add_input(self.crossover_value.into());

        let crossover_blinding_factor = self.crossover_blinding_factor.into();
        let crossover_blinding_factor =
            composer.add_input(crossover_blinding_factor);

        let crossover_value_commitment_p = gadgets::commitment(
            composer,
            crossover_value,
            crossover_blinding_factor,
        );

        let crossover_value_commitment =
            self.crossover.value_commitment().into();
        composer.assert_equal_public_point(
            crossover_value_commitment_p,
            crossover_value_commitment,
        );

        let crossover_value_commitment = crossover_value_commitment_p;

        //  2. range(vc,64)
        composer.range_gate(crossover_value, 64);

        //  3. commitment(Cm,vm,bm)
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

        //  4. range(vm,64)
        composer.range_gate(message_value, 64);

        //  5. Da:=select(fm,I,Dap,Das)
        let identity = Point::identity(composer);
        let message_derive_key_is_public =
            composer.add_input(self.message_derive_key_is_public);

        let message_derive_key_public_a =
            composer.add_public_affine(self.message_derive_key_public_a.into());
        let message_derive_key_private_a =
            composer.add_affine(self.message_derive_key_private_a.into());

        let message_derive_key_a = gadgets::identity_select_point(
            composer,
            message_derive_key_is_public,
            identity,
            message_derive_key_public_a,
            message_derive_key_private_a,
        );

        //  6. Db:=select(fm,I,Dbp,Dbs)
        let message_derive_key_public_b =
            composer.add_public_affine(self.message_derive_key_public_b.into());
        let message_derive_key_private_b =
            composer.add_affine(self.message_derive_key_private_b.into());

        let message_derive_key_b = gadgets::identity_select_point(
            composer,
            message_derive_key_is_public,
            identity,
            message_derive_key_public_b,
            message_derive_key_private_b,
        );

        //  7. Km:=stealthAddress(mr,Da,Db)
        let message_r = composer.add_input(self.message_r.into());
        let message_stealth_address = gadgets::stealth_address(
            composer,
            message_r,
            message_derive_key_a,
            message_derive_key_b,
        );

        composer.assert_equal_public_point(
            message_stealth_address,
            self.message_pk_r.into(),
        );

        //  8. es:=mr·Da
        let cipher_secret =
            composer.variable_base_scalar_mul(message_r, message_derive_key_a);

        //  9. Em==encrypt(es,Nm,[vm,bm])
        let message_nonce = self.message.nonce().clone().into();
        let message_nonce_p = composer.add_input(message_nonce);
        composer.constrain_to_constant(
            message_nonce_p,
            BlsScalar::zero(),
            Some(-message_nonce),
        );
        let message_nonce = message_nonce_p;

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

        // 10. Hs==H([Cc,Nc,Ec,Cm,Nm,Em,Ac])
        let address = composer.add_input(self.address);
        composer.constrain_to_constant(
            address,
            BlsScalar::zero(),
            Some(-self.address),
        );

        let crossover_nonce = self.crossover.nonce().clone().into();
        let crossover_nonce_p = composer.add_input(crossover_nonce);
        composer.constrain_to_constant(
            crossover_nonce_p,
            BlsScalar::zero(),
            Some(-crossover_nonce),
        );
        let crossover_nonce = crossover_nonce_p;

        let mut inputs = vec![
            *crossover_value_commitment.x(),
            *crossover_value_commitment.y(),
            crossover_nonce,
        ];

        let crossover_cipher =
            self.crossover.encrypted_data().cipher().iter().map(|c| {
                let c = *c;

                let c_p = composer.add_input(c);
                composer.constrain_to_constant(
                    c_p,
                    BlsScalar::zero(),
                    Some(-c),
                );

                c_p
            });
        inputs.extend(crossover_cipher);

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

        // 11. schnorrVerify(σ,Kf,H)
        let fee_pk_r = self.fee.stealth_address().pk_r().as_ref().into();
        let fee_pk_r = composer.add_public_affine(fee_pk_r);

        let r = composer.add_affine(self.signature.R().into());
        let u = *self.signature.u();
        let u = composer.add_input(u.into());

        dusk_schnorr::gadgets::single_key_verify(
            composer, r, u, fee_pk_r, hash,
        );

        // 12. vc−vm=0
        composer.assert_equal(crossover_value, message_value);

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 14
    }
}
