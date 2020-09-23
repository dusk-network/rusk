// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.



use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;

/// Prove knowledge of the preimage of a note,
/// used as input for a transaction.
#[allow(non_snake_case)]
pub fn input_preimage(
    composer: &mut StandardComposer,
    value_commitment_x: AllocatedScalar,
    value_commitment_y: AllocatedScalar,
    pos: AllocatedScalar,
    pk_r_x: AllocatedScalar,
    pk_r_y: AllocatedScalar,
    note_hash: BlsScalar,
) {
    let output = sponge_hash_gadget(
        composer,
        &[
            value_commitment_x.var,
            value_commitment_y.var,
            pos.var,
            pk_r_x.var,
            pk_r_y.var,
        ],
    );

    composer.constrain_to_constant(output, BlsScalar::zero(), -note_hash);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_pki::{Ownable, PublicSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;
    use phoenix_core::{Note, NoteType};
    use rand::Rng;

    #[test]
    fn preimage_gadget() -> Result<(), Error> {
        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let preimage_circuit = |composer: &mut StandardComposer,
                                note: &Note| {
            let commitment: Vec<AllocatedScalar> = note
                .value_commitment()
                .to_hash_inputs()
                .iter()
                .map(|coord| AllocatedScalar::allocate(composer, *coord))
                .collect();
            let pk: Vec<AllocatedScalar> = note
                .stealth_address()
                .pk_r()
                .to_hash_inputs()
                .iter()
                .map(|coord| AllocatedScalar::allocate(composer, *coord))
                .collect();
            let pos = AllocatedScalar::allocate(
                composer,
                BlsScalar::from(note.pos()),
            );
            let note_hash = note.hash();
            input_preimage(
                composer,
                commitment[0],
                commitment[1],
                pos,
                pk[0],
                pk[1],
                note_hash,
            );
        };
        let mut prover = Prover::new(b"test");

        let value: u64 = rand::thread_rng().gen();
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let sk2 = JubJubScalar::random(&mut rand::thread_rng());
        let pk = GENERATOR_EXTENDED * sk;
        let pk2 = GENERATOR_EXTENDED * sk2;
        let key = &PublicSpendKey::new(pk, pk2);

        let note = Note::new(NoteType::Transparent, key, value);

        preimage_circuit(prover.mut_cs(), &note);
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");
        preimage_circuit(verifier.mut_cs(), &note);
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
