// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use std::convert::TryInto;

/// Prove knowledge of the preimage of a note,
/// used as input for a transaction.
#[allow(non_snake_case)]
pub fn input_preimage(
    composer: &mut StandardComposer,
    note_type: AllocatedScalar,
    value_commitment_x: AllocatedScalar,
    value_commitment_y: AllocatedScalar,
    pk_r_x: AllocatedScalar,
    pk_r_y: AllocatedScalar,
    randomness_x: AllocatedScalar,
    randomness_y: AllocatedScalar,
    position: AllocatedScalar,
    cipher_one: AllocatedScalar,
    cipher_two: AllocatedScalar,
    cipher_three: AllocatedScalar,
) -> Variable {
    let output = sponge_hash_gadget(
        composer,
        &[
            note_type.var,
            value_commitment_x.var,
            value_commitment_y.var,
            pk_r_x.var,
            pk_r_y.var,
            randomness_x.var,
            randomness_y.var,
            position.var,
            cipher_one.var,
            cipher_two.var,
            cipher_three.var,
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
        let note_bytes = note.to_bytes();
        let cipher = &note_bytes[note_bytes.len() - 96..];
        let arr1: [u8; 32] = cipher[..32].try_into().expect("Invalid length");
        let cipher1 = BlsScalar::from_bytes(&arr1).unwrap();
        let arr2: [u8; 32] = cipher[32..64].try_into().expect("Invalid length");
        let cipher2 = BlsScalar::from_bytes(&arr2).unwrap();
        let arr3: [u8; 32] = cipher[64..].try_into().expect("Invalid length");
        let cipher3 = BlsScalar::from_bytes(&arr3).unwrap();

        let note_hash = note.hash();

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 16)?;
        let mut prover = Prover::new(b"test");

        let note_type = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(note.note() as u64),
        );
        let comm_x = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.value_commitment().to_hash_inputs()[0],
        );
        let comm_y = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.value_commitment().to_hash_inputs()[1],
        );
        let pkr_x = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[0],
        );
        let pkr_y = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[1],
        );
        let r_x = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().R().to_hash_inputs()[0],
        );
        let r_y = AllocatedScalar::allocate(
            prover.mut_cs(),
            note.stealth_address().R().to_hash_inputs()[1],
        );
        let pos = AllocatedScalar::allocate(
            prover.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let cipher_1 = AllocatedScalar::allocate(prover.mut_cs(), cipher1);
        let cipher_2 = AllocatedScalar::allocate(prover.mut_cs(), cipher2);
        let cipher_3 = AllocatedScalar::allocate(prover.mut_cs(), cipher3);

        let a = input_preimage(
            prover.mut_cs(),
            note_type,
            comm_x,
            comm_y,
            pkr_x,
            pkr_y,
            r_x,
            r_y,
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
        prover.mut_cs().check_circuit_satisfied();

        let mut verifier = Verifier::new(b"test");
        let note_type = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(note.note() as u64),
        );
        let comm_x = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.value_commitment().to_hash_inputs()[0],
        );
        let comm_y = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.value_commitment().to_hash_inputs()[1],
        );
        let pkr_x = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[0],
        );
        let pkr_y = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().pk_r().to_hash_inputs()[1],
        );
        let r_x = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().R().to_hash_inputs()[0],
        );
        let r_y = AllocatedScalar::allocate(
            verifier.mut_cs(),
            note.stealth_address().R().to_hash_inputs()[1],
        );
        let pos = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(note.pos()),
        );
        let cipher_1 = AllocatedScalar::allocate(verifier.mut_cs(), cipher1);
        let cipher_2 = AllocatedScalar::allocate(verifier.mut_cs(), cipher2);
        let cipher_3 = AllocatedScalar::allocate(verifier.mut_cs(), cipher3);

        let a = input_preimage(
            verifier.mut_cs(),
            note_type,
            comm_x,
            comm_y,
            pkr_x,
            pkr_y,
            r_x,
            r_y,
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
