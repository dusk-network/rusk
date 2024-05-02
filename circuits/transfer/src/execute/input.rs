// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_merkle::Aggregate;
use phoenix_core::{Note, NoteType, Ownable, PublicKey};
use poseidon_merkle::{Item, Opening, Tree};

use dusk_plonk::prelude::*;

mod signature;
mod witness;

pub use signature::CircuitInputSignature;
pub use witness::WitnessInput;

#[derive(Debug, Clone)]
pub struct CircuitInput<T, const H: usize, const A: usize> {
    branch: Opening<T, H, A>,
    note: Note,
    note_pk: JubJubAffine,
    note_pk_p: JubJubAffine,
    value: u64,
    blinding_factor: JubJubScalar,
    nullifier: BlsScalar,
    signature: CircuitInputSignature,
}

impl<T, const H: usize, const A: usize> CircuitInput<T, H, A> {
    pub fn new(
        branch: Opening<T, H, A>,
        note: Note,
        note_pk_p: JubJubAffine,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
        signature: CircuitInputSignature,
    ) -> Self {
        let note_pk = note.stealth_address().pk_r().as_ref().into();

        Self {
            branch,
            note,
            note_pk,
            note_pk_p,
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

    pub const fn branch(&self) -> &Opening<T, H, A> {
        &self.branch
    }

    pub const fn nullifier(&self) -> &BlsScalar {
        &self.nullifier
    }

    pub fn to_witness(
        &self,
        composer: &mut Composer,
    ) -> Result<WitnessInput, Error> {
        let nullifier = self.nullifier;

        let note = self.note;

        let note_pk = composer.append_point(self.note_pk);
        let note_pk_p = composer.append_point(self.note_pk_p);

        let note_hash = note.hash();
        let note_hash = composer.append_witness(note_hash);

        let hash_inputs = note.hash_inputs();

        let note_type = hash_inputs[0];
        let note_type = composer.append_witness(note_type);

        // Plonk API will not allow points to be constructed from variables
        let value_commitment = note.value_commitment();
        let value_commitment = composer.append_point(value_commitment);

        let pos = hash_inputs[5];
        let pos = composer.append_witness(pos);

        let value = composer.append_witness(self.value);
        let blinding_factor = composer.append_witness(self.blinding_factor);

        let signature = &self.signature;
        let schnorr_u = BlsScalar::from(*signature.u());
        let schnorr_u = composer.append_witness(schnorr_u);
        let schnorr_r = composer.append_point(*signature.r());
        let schnorr_r_p = composer.append_point(*signature.r_p());

        Ok(WitnessInput {
            note_pk,
            note_pk_p,
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

impl<T, const H: usize, const A: usize> CircuitInput<T, H, A>
where
    T: Aggregate<A> + Clone + Default,
{
    pub fn pad() -> Self {
        let mut tree = Tree::new();
        tree.insert(0, Item::new(BlsScalar::default(), T::default()));
        let opening = tree.opening(0).expect(
            "It should be possible to create an opening at the given position",
        );

        let note = Note::deterministic(
            NoteType::Transparent,
            &JubJubScalar::default(),
            BlsScalar::default(),
            &PublicKey::new(
                JubJubExtended::default(),
                JubJubExtended::default(),
            ),
            u64::default(),
            JubJubScalar::default(),
        );

        Self::new(
            opening,
            note,
            JubJubAffine::default(),
            u64::default(),
            JubJubScalar::default(),
            BlsScalar::default(),
            CircuitInputSignature::default(),
        )
    }
}
