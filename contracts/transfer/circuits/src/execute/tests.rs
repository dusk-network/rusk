// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::tests::leaf::NoteLeaf;
use crate::ExecuteCircuit;

use anyhow::Result;
use canonical_host::MemStore;
use dusk_pki::{Ownable, SecretSpendKey};
use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
use phoenix_core::Note;
use poseidon252::tree::{PoseidonAnnotation, PoseidonTree};
use rand::rngs::StdRng;
use rand::SeedableRng;

use dusk_plonk::prelude::*;

use std::convert::TryInto;

#[test]
// This test ensures the execute gadget is done correctly
// by creating two notes and setting their field values
// in the execute circuit
fn execute() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let mut tree =
        PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

    let mut circuit = ExecuteCircuit::with_size(1 << 16);

    let a_ssk = SecretSpendKey::random(&mut rng);
    let a_psk = a_ssk.public_key();
    let a_value = 600;
    let a_note = Note::transparent(&mut rng, &a_psk, a_value);
    let a_blinding_factor = a_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );

    let p = tree.push(a_note.into()).expect("Tree append error");
    let a_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");

    let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
    let a_nullifier = a_note.gen_nullifier(&a_ssk);
    circuit.add_input(
        &mut rng,
        &tree,
        a_sk_r,
        a_note,
        a_value,
        a_blinding_factor,
        a_nullifier,
    )?;

    let b_ssk = SecretSpendKey::random(&mut rng);
    let b_psk = b_ssk.public_key();
    let b_value = 450;
    let b_blinding_factor = JubJubScalar::random(&mut rng);
    let b_note = Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
    circuit.add_output(b_note, b_value, b_blinding_factor);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();
    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();
    circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

    let d_ssk = SecretSpendKey::random(&mut rng);
    let d_psk = d_ssk.public_key();
    let d_value = 750;
    let d_note = Note::transparent(&mut rng, &d_psk, d_value);
    let d_blinding_factor = d_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );
    circuit.add_output(d_note, d_value, d_blinding_factor);

    let e_ssk = SecretSpendKey::random(&mut rng);
    let e_psk = e_ssk.public_key();
    let e_value = 700;
    let e_blinding_factor = JubJubScalar::random(&mut rng);
    let e_note = Note::obfuscated(&mut rng, &e_psk, e_value, e_blinding_factor);

    let p = tree.push(e_note.into()).expect("Tree append error");
    let e_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");

    let e_sk_r = e_ssk.sk_r(e_note.stealth_address());
    let e_nullifier = e_note.gen_nullifier(&e_ssk);
    circuit.add_input(
        &mut rng,
        &tree,
        e_sk_r,
        e_note,
        e_value,
        e_blinding_factor,
        e_nullifier,
    )?;

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )?;

    let (pk, vk) = circuit.compile(&pp)?;
    circuit.get_mut_pi_positions().clear();

    let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
    let pi = circuit.get_pi_positions().clone();

    circuit.verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
}

#[test]
// This test ensures the execute gadget is done correctly
// by creating two notes and setting their field values
// in the execute circuit
fn wrong_note_value_one() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let mut tree =
        PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

    let mut circuit = ExecuteCircuit::with_size(1 << 15);

    let a_ssk = SecretSpendKey::random(&mut rng);
    let a_psk = a_ssk.public_key();
    let a_value = 600;
    let a_note = Note::transparent(&mut rng, &a_psk, a_value);
    let a_blinding_factor = a_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );

    let p = tree.push(a_note.into()).expect("Tree append error");
    let a_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");

    let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
    let a_nullifier = a_note.gen_nullifier(&a_ssk);
    circuit.add_input(
        &mut rng,
        &tree,
        a_sk_r,
        a_note,
        a_value,
        a_blinding_factor,
        a_nullifier,
    )?;

    let b_ssk = SecretSpendKey::random(&mut rng);
    let b_psk = b_ssk.public_key();
    let b_value = 150;
    let b_blinding_factor = JubJubScalar::random(&mut rng);
    let b_note = Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
    circuit.add_output(b_note, b_value, b_blinding_factor);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();
    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();
    circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

    let d_ssk = SecretSpendKey::random(&mut rng);
    let d_psk = d_ssk.public_key();
    let d_value_note = 351;
    let d_value_circuit = 350;
    let d_note = Note::transparent(&mut rng, &d_psk, d_value_note);
    let d_blinding_factor = d_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );
    circuit.add_output(d_note, d_value_circuit, d_blinding_factor);

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )?;

    let (pk, vk) = circuit.compile(&pp)?;
    circuit.get_mut_pi_positions().clear();

    let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
        .is_ok();
    assert!(!verify);

    Ok(())
}

#[test]
// This circuit tests to see if a wrong nullifier
// leads to a failed circuit
fn wrong_nullifier() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let mut tree =
        PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

    let mut circuit = ExecuteCircuit::with_size(1 << 15);

    let a_ssk = SecretSpendKey::random(&mut rng);
    let a_psk = a_ssk.public_key();
    let a_value = 600;
    let a_note = Note::transparent(&mut rng, &a_psk, a_value);
    let a_blinding_factor = a_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );

    let p = tree.push(a_note.into()).expect("Tree append error");
    let a_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");

    let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
    let mut a_nullifier = a_note.gen_nullifier(&a_ssk);
    a_nullifier += BlsScalar::one();
    circuit.add_input(
        &mut rng,
        &tree,
        a_sk_r,
        a_note,
        a_value,
        a_blinding_factor,
        a_nullifier,
    )?;

    let b_ssk = SecretSpendKey::random(&mut rng);
    let b_psk = b_ssk.public_key();
    let b_value = 150;
    let b_blinding_factor = JubJubScalar::random(&mut rng);
    let b_note = Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
    circuit.add_output(b_note, b_value, b_blinding_factor);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();
    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();
    circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

    let d_ssk = SecretSpendKey::random(&mut rng);
    let d_psk = d_ssk.public_key();
    let d_value = 350;
    let d_note = Note::transparent(&mut rng, &d_psk, d_value);
    let d_blinding_factor = d_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );
    circuit.add_output(d_note, d_value, d_blinding_factor);

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )?;

    let (pk, vk) = circuit.compile(&pp)?;
    circuit.get_mut_pi_positions().clear();

    let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
        .is_ok();
    assert!(!verify);

    Ok(())
}

#[test]
// The fee is a public input and is the value
// paid for processing a transaction. With an
// incorrect value for PI, the test should fail.
fn wrong_fee() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let mut tree =
        PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

    let mut circuit = ExecuteCircuit::with_size(1 << 15);

    let a_ssk = SecretSpendKey::random(&mut rng);
    let a_psk = a_ssk.public_key();
    let a_value = 600;
    let a_note = Note::transparent(&mut rng, &a_psk, a_value);
    let a_blinding_factor = a_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );

    let p = tree.push(a_note.into()).expect("Tree append error");
    let a_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");

    let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
    let a_nullifier = a_note.gen_nullifier(&a_ssk);
    circuit.add_input(
        &mut rng,
        &tree,
        a_sk_r,
        a_note,
        a_value,
        a_blinding_factor,
        a_nullifier,
    )?;

    let b_ssk = SecretSpendKey::random(&mut rng);
    let b_psk = b_ssk.public_key();
    let b_value = 150;
    let b_blinding_factor = JubJubScalar::random(&mut rng);
    let b_note = Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
    circuit.add_output(b_note, b_value, b_blinding_factor);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();
    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();
    circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

    let d_ssk = SecretSpendKey::random(&mut rng);
    let d_psk = d_ssk.public_key();
    let d_value = 350;
    let d_note = Note::transparent(&mut rng, &d_psk, d_value);
    let d_blinding_factor = d_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );
    circuit.add_output(d_note, d_value, d_blinding_factor);

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )?;

    let (pk, vk) = circuit.compile(&pp)?;
    circuit.get_mut_pi_positions().clear();

    let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
    let mut pi = circuit.get_pi_positions().clone();

    let fee = BlsScalar::from(c_value);
    match &mut pi[2] {
        PublicInput::BlsScalar(f, _) if f == &fee => *f += BlsScalar::one(),
        _ => panic!("Unexpected public input!"),
    }

    let verify = circuit
        .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
        .is_ok();
    assert!(!verify);

    Ok(())
}

#[test]
// This test pushes the position of the note,
// after the note position is pushed to the tree.
// This should fail meaning the user cannot amend
// the position of the note in the tree after its
// set.
fn pushing_note_to_wrong_position() -> Result<()> {
    let mut rng = StdRng::seed_from_u64(2324u64);

    let mut tree =
        PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

    let mut circuit = ExecuteCircuit::with_size(1 << 15);

    let a_ssk = SecretSpendKey::random(&mut rng);
    let a_psk = a_ssk.public_key();
    let a_value = 600;
    let a_note = Note::transparent(&mut rng, &a_psk, a_value);
    let a_blinding_factor = a_note.blinding_factor(None).expect(
        "Failed to extract the blinding factor from a transparent note",
    );

    let p = tree.push(a_note.into()).expect("Tree append error");
    let mut a_note = tree
        .get(p)
        .expect("Tree fetch error")
        .map(|n| Note::from(n))
        .expect("a_note not found!");
    let pos = a_note.pos();
    a_note.set_pos(pos + 1);

    let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
    let a_nullifier = a_note.gen_nullifier(&a_ssk);
    circuit
        .add_input(
            &mut rng,
            &tree,
            a_sk_r,
            a_note,
            a_value,
            a_blinding_factor,
            a_nullifier,
        )
        .unwrap_or(());

    let b_ssk = SecretSpendKey::random(&mut rng);
    let b_psk = b_ssk.public_key();
    let b_value = 150;
    let b_blinding_factor = JubJubScalar::random(&mut rng);
    let b_note = Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
    circuit.add_output(b_note, b_value, b_blinding_factor);

    let c_ssk = SecretSpendKey::random(&mut rng);
    let c_psk = c_ssk.public_key();
    let c_value = 100;
    let c_blinding_factor = JubJubScalar::random(&mut rng);
    let c_note = Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
    let (_, crossover) = c_note
        .try_into()
        .expect("Failed to generate fee and crossover!");
    let c_value_commitment = *crossover.value_commitment();
    circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

    let d_ssk = SecretSpendKey::random(&mut rng);
    let d_psk = d_ssk.public_key();
    let d_value = 350;
    let d_note = Note::transparent(&mut rng, &d_psk, d_value);
    let d_blinding_factor = d_note
        .blinding_factor(None)
        .expect("Failed to extract blinding_factor from a transparent note.");
    circuit.add_output(d_note, d_value, d_blinding_factor);

    // Generate Composer & Public Parameters
    let pp = PublicParameters::setup(
        circuit.get_trim_size(),
        &mut rand::thread_rng(),
    )?;

    let (pk, vk) = circuit.compile(&pp)?;
    circuit.get_mut_pi_positions().clear();

    let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
    let pi = circuit.get_pi_positions().clone();

    let verify = circuit
        .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
        .is_ok();
    assert!(!verify);

    Ok(())
}
