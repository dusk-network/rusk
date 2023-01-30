// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::POSEIDON_TREE_DEPTH;

use dusk_pki::Ownable;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;

use dusk_plonk::prelude::*;

mod signature;
mod witness;

pub use signature::CircuitInputSignature;
pub use witness::WitnessInput;

#[derive(Debug, Clone)]
pub struct CircuitInput {
    branch: PoseidonBranch<POSEIDON_TREE_DEPTH>,
    note: Note,
    pk_r: JubJubAffine,
    pk_r_p: JubJubAffine,
    value: u64,
    blinding_factor: JubJubScalar,
    nullifier: BlsScalar,
    signature: CircuitInputSignature,
}

impl CircuitInput {
    pub fn new(
        branch: PoseidonBranch<POSEIDON_TREE_DEPTH>,
        note: Note,
        pk_r_p: JubJubAffine,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
        signature: CircuitInputSignature,
    ) -> Self {
        let pk_r = note.stealth_address().pk_r().as_ref().into();

        Self {
            branch,
            note,
            pk_r,
            pk_r_p,
            value,
            blinding_factor,
            nullifier,
            signature,
        }
    }

    pub const fn signature(&self) -> &CircuitInputSignature {
        &self.signature
    }

    pub const fn note(&self) -> &Note {
        &self.note
    }

    pub const fn branch(&self) -> &PoseidonBranch<POSEIDON_TREE_DEPTH> {
        &self.branch
    }

    pub const fn nullifier(&self) -> &BlsScalar {
        &self.nullifier
    }

    pub fn to_witness<C: Composer>(
        &self,
        composer: &mut C,
    ) -> Result<WitnessInput, Error> {
        let nullifier = self.nullifier;

        let note = self.note;

        let pk_r = composer.append_point(self.pk_r);
        let pk_r_p = composer.append_point(self.pk_r_p);

        let note_hash = note.hash();
        let note_hash = composer.append_witness(note_hash);

        let hash_inputs = note.hash_inputs();

        let note_type = hash_inputs[0];
        let note_type = composer.append_public(note_type);

        // Plonk API will not allow points to be constructed from variables
        let value_commitment = note.value_commitment();
        let value_commitment = composer.append_point(value_commitment);

        let pos = hash_inputs[8];
        let pos = composer.append_witness(pos);

        let value = composer.append_witness(self.value);
        let blinding_factor = composer.append_witness(self.blinding_factor);

        let signature = &self.signature;
        let schnorr_u = BlsScalar::from(*signature.u());
        let schnorr_u = composer.append_witness(schnorr_u);
        let schnorr_r = composer.append_point(*signature.r());
        let schnorr_r_p = composer.append_point(*signature.r_p());

        Ok(WitnessInput {
            pk_r,
            pk_r_p,
            note_hash,

            note_type,
            value_commitment,
            pos,

            value,
            blinding_factor,

            schnorr_u,
            schnorr_r,
            schnorr_r_p,

            nullifier,
        })
    }
}
