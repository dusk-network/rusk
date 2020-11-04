// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::AffinePoint;
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::{sponge_hash, sponge_hash_gadget};


/// Prove knowledge of the preimage of a note,
/// used as input for a transaction.
#[allow(non_snake_case)]
pub fn input_preimage(
    composer: &mut StandardComposer,
    note_type: AllocatedScalar,
    value_commitment: PlonkPoint,
    nonce: AllocatedScalar,
    pk_r: PlonkPoint,
    randomness: PlonkPoint,
    position: AllocatedScalar,
    cipher_one: AllocatedScalar,
    cipher_two: AllocatedScalar,
    cipher_three: AllocatedScalar,
) -> Variable {
    sponge_hash_gadget(
        composer,
        &[
            note_type.var,
            *value_commitment.x(),
            *value_commitment.y(),
            nonce.var,
            *pk_r.x(),
            *pk_r.y(),
            *randomness.x(),
            *randomness.y(),
            position.var,
            cipher_one.var,
            cipher_two.var,
            cipher_three.var,
        ],
    )

}

#[cfg(test)]
mod preimage_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_pki::{Ownable, PublicSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use phoenix_core::{Note, NoteType};

    #[test]
    fn preimage_gadget() -> Result<(), Error> {
        let value = 7 as u64;
        let blinder = JubJubScalar::from(1123 as u64);
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let pk1 = GENERATOR_EXTENDED * secret1;
        let pk2 = GENERATOR_EXTENDED * secret2;
        let psk = PublicSpendKey::new(pk1, pk2);
        let r = JubJubScalar::from(112 as u64);
        let nonce = JubJubScalar::from(345 as u64);
        let note = Note::deterministic(
            NoteType::Obfuscated,
            &r,
            nonce,
            &psk,
            value,
            blinder,
        );

        let note_hash = note.hash();

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let mut prover = Prover::new(b"test");

        let note_type = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(note.note() as u64),
        );
        let commitment = PlonkPoint::from_private_affine(
            prover.mut_cs(),
            AffinePoint::from(note.value_commitment()),
        );
        let nonce = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(*note.nonce()),
        );
        let pkr = PlonkPoint::from_private_affine(
            prover.mut_cs(),
            AffinePoint::from(note.stealth_address().pk_r()),
        );
        let r = PlonkPoint::from_private_affine(
            prover.mut_cs(),
            AffinePoint::from(note.stealth_address().R()),
        );
        let pos = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let cipher_1 =
            AllocatedScalar::allocate(prover.mut_cs(), note.cipher()[0]);
        let cipher_2 =
            AllocatedScalar::allocate(prover.mut_cs(), note.cipher()[1]);
        let cipher_3 =
            AllocatedScalar::allocate(prover.mut_cs(), note.cipher()[2]);

        let a = input_preimage(
            prover.mut_cs(),
            note_type,
            commitment,
            nonce,
            pkr,
            r,
            pos,
            cipher_1,
            cipher_2,
            cipher_3,
        );
        prover
            .mut_cs()
            .constrain_to_constant(a, BlsScalar::zero(), -note_hash);
        let prover_pi = prover.mut_cs().public_inputs.clone();
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");
        let note_type = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(note.note() as u64),
        );
        let commitment = PlonkPoint::from_private_affine(
            verifier.mut_cs(),
            AffinePoint::from(note.value_commitment()),
        );
        let nonce = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(*note.nonce()),
        );
        let pkr = PlonkPoint::from_private_affine(
            verifier.mut_cs(),
            AffinePoint::from(note.stealth_address().pk_r()),
        );
        let r = PlonkPoint::from_private_affine(
            verifier.mut_cs(),
            AffinePoint::from(note.stealth_address().R()),
        );
        let pos = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let cipher_1 =
            AllocatedScalar::allocate(verifier.mut_cs(), note.cipher()[0]);
        let cipher_2 =
            AllocatedScalar::allocate(verifier.mut_cs(), note.cipher()[1]);
        let cipher_3 =
            AllocatedScalar::allocate(verifier.mut_cs(), note.cipher()[2]);

        let a = input_preimage(
            verifier.mut_cs(),
            note_type,
            commitment,
            nonce,
            pkr,
            r,
            pos,
            cipher_1,
            cipher_2,
            cipher_3,
        );
        verifier.mut_cs().constrain_to_constant(
            a,
            BlsScalar::zero(),
            -note_hash,
        );
        verifier.preprocess(&ck)?;
        verifier.verify(&proof, &vk, &prover_pi)
    }
}
