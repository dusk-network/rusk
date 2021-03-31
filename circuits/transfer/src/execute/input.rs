// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::SIGN_MESSAGE;

use dusk_pki::Ownable;
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base;
use dusk_plonk::constraint_system::ecc::Point;
use dusk_plonk::jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;
use schnorr::Proof as SchnorrProof;

use dusk_plonk::prelude::*;

/// Coupled code
///
/// Currently, Plonk is not a dependency of phoenix-core. This means the circuit
/// construction of the note must be done here.
///
/// Ideally, there would be a `fn hash_inputs_witness(&self, composer)`
/// implemented for `Note`.
///
/// Since the circuit will perform a pre-image check over the result of this
/// function, the structure is safe
///
/// However, if `Note::hash_inputs` ever change, this circuit will be broken
#[derive(Debug, Clone, Copy)]
pub struct WitnessInput {
    pub sk_r: Variable,
    pub pk_r: Point,
    pub note_hash: Variable,

    pub note_type: Variable,
    pub value_commitment: Point,
    pub nonce: Variable,
    pub r: Point,
    pub pos: Variable,
    pub cipher: [Variable; PoseidonCipher::cipher_size()],

    pub value: Variable,
    pub blinding_factor: Variable,

    pub pk_r_prime: Point,
    pub schnorr_message: Variable,
    pub schnorr_u: Variable,
    pub schnorr_r: Point,
    pub schnorr_r_prime: Point,

    // Public data
    pub nullifier: BlsScalar,
}

impl WitnessInput {
    pub fn to_hash_inputs(&self) -> [Variable; 12] {
        [
            self.note_type,
            *self.value_commitment.x(),
            *self.value_commitment.y(),
            self.nonce,
            *self.pk_r.x(),
            *self.pk_r.y(),
            *self.r.x(),
            *self.r.y(),
            self.pos,
            self.cipher[0],
            self.cipher[1],
            self.cipher[2],
        ]
    }
}

pub const POSEIDON_BRANCH_DEPTH: usize = 17;

#[derive(Debug, Clone)]
pub struct CircuitInput {
    sk_r: JubJubScalar,
    branch: PoseidonBranch<POSEIDON_BRANCH_DEPTH>,
    note: Note,
    value: u64,
    blinding_factor: JubJubScalar,
    signature: SchnorrProof,
    nullifier: BlsScalar,
}

impl CircuitInput {
    pub fn new(
        signature: SchnorrProof,
        branch: PoseidonBranch<POSEIDON_BRANCH_DEPTH>,
        sk_r: JubJubScalar,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
    ) -> Self {
        Self {
            sk_r,
            branch,
            note,
            value,
            blinding_factor,
            signature,
            nullifier,
        }
    }

    pub const fn branch(&self) -> &PoseidonBranch<POSEIDON_BRANCH_DEPTH> {
        &self.branch
    }

    pub const fn nullifier(&self) -> &BlsScalar {
        &self.nullifier
    }

    pub fn to_witness(&self, composer: &mut StandardComposer) -> WitnessInput {
        let nullifier = self.nullifier;

        let note = self.note;

        let sk_r = self.sk_r;
        let sk_r = composer.add_input(sk_r.into());

        let pk_r = fixed_base::scalar_mul(composer, sk_r, GENERATOR_EXTENDED);
        let pk_r = *pk_r.point();

        let note_hash = note.hash();
        let note_hash = composer.add_input(note_hash);

        let hash_inputs = note.hash_inputs();

        let note_type = hash_inputs[0];
        let note_type = composer.add_input(note_type);

        // Plonk API will not allow points to be constructed from variables
        let value_commitment = note.value_commitment().into();
        let value_commitment =
            Point::from_private_affine(composer, value_commitment);

        let nonce = hash_inputs[3];
        let nonce = composer.add_input(nonce);

        let r = note.stealth_address().R().into();
        let r = Point::from_private_affine(composer, r);

        let pos = hash_inputs[8];
        let pos = composer.add_input(pos);

        let mut cipher = [pos; 3];
        cipher
            .iter_mut()
            .zip(hash_inputs[9..].iter())
            .for_each(|(c, i)| {
                *c = composer.add_input(*i);
            });

        let value = composer.add_input(self.value.into());
        let blinding_factor = composer.add_input(self.blinding_factor.into());

        let pk_r_prime =
            fixed_base::scalar_mul(composer, sk_r, GENERATOR_NUMS_EXTENDED);
        let pk_r_prime = *pk_r_prime.point();
        let schnorr_message = SIGN_MESSAGE;
        let schnorr_message =
            composer.add_witness_to_circuit_description(schnorr_message);
        let schnorr_u = *self.signature.u();
        let schnorr_u = composer.add_input(schnorr_u.into());
        let schnorr_r = self.signature.keys().R().as_ref().into();
        let schnorr_r = Point::from_private_affine(composer, schnorr_r);
        let schnorr_r_prime = self.signature.keys().R_prime().as_ref().into();
        let schnorr_r_prime =
            Point::from_private_affine(composer, schnorr_r_prime);

        WitnessInput {
            sk_r,
            pk_r,
            note_hash,

            note_type,
            value_commitment,
            nonce,
            r,
            pos,
            cipher,

            value,
            blinding_factor,

            pk_r_prime,
            schnorr_message,
            schnorr_u,
            schnorr_r,
            schnorr_r_prime,

            nullifier,
        }
    }
}
