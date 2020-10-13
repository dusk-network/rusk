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
    type: AllocatedScalar,
    value_commitment_x: AllocatedScalar,
    value_commitment_y: AllocatedScalar,
    pos: AllocatedScalar,
    pk_r_x: AllocatedScalar,
    pk_r_y: AllocatedScalar,
    encrypted_data: AllocatedScalar,
) -> Variable {
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

    output
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_pki::{Ownable, PublicSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use phoenix_core::{Note, NoteType};

    #[test]
    fn preimage_gadget() -> Result<(), Error> {
        let secret1 = JubJubScalar::from(100u64);
        let secret2 = JubJubScalar::from(200u64);
        let pk1 = GENERATOR_EXTENDED * secret1;
        let pk2 = GENERATOR_EXTENDED * secret2;
        let psk = PublicSpendKey::new(pk1, pk2);
        let value = 25u64;
        let note = Note::new(NoteType::Transparent, &psk, value);
        let note_hash = note.hash();

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 15, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 14)?;
        let mut prover = Prover::new(b"test");

        let comm_x = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.value_commitment().to_hash_inputs()[0],
        );
        let comm_y = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.value_commitment().to_hash_inputs()[1],
        );
        let pos = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let pkr_x = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[0],
        );
        let pkr_y = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[1],
        );

        let a =
            input_preimage(prover.mut_cs(), comm_x, comm_y, pos, pkr_x, pkr_y);
        prover
            .mut_cs()
            .constrain_to_constant(a, BlsScalar::zero(), -note_hash);
        let prover_pi = prover.mut_cs().public_inputs.clone();
        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");
        let comm_x = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.value_commitment().to_hash_inputs()[0],
        );
        let comm_y = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.value_commitment().to_hash_inputs()[1],
        );
        let pos = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let pkr_x = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[0],
        );
        let pkr_y = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[1],
        );

        let a = input_preimage(
            verifier.mut_cs(),
            comm_x,
            comm_y,
            pos,
            pkr_x,
            pkr_y,
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
