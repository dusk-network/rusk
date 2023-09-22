// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::{PublicSpendKey, SecretSpendKey};
use phoenix_core::Note;
use poseidon_merkle::{Item, Tree};
use transfer_circuits::Error;
use transfer_circuits::{
    ExecuteCircuit, ExecuteCircuitFourTwo, ExecuteCircuitOneTwo,
    ExecuteCircuitThreeTwo, ExecuteCircuitTwoTwo,
};

use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

mod keys;
use keys::load_keys;

const HEIGHT: usize = 17;
const ARITY: usize = 4;

struct WrappedNote(Note);

impl<T> From<WrappedNote> for Item<T>
where
    T: Default,
{
    fn from(note: WrappedNote) -> Self {
        Self {
            hash: dusk_poseidon::sponge::hash(&note.0.hash_inputs()),
            data: T::default(),
        }
    }
}

pub fn create_test_note<R: RngCore + CryptoRng>(
    rng: &mut R,
    psk: &PublicSpendKey,
    transparent: bool,
    value: u64,
) -> Note {
    if transparent {
        Note::transparent(rng, psk, value)
    } else {
        let blinding_factor = JubJubScalar::random(rng);

        Note::obfuscated(rng, psk, value, blinding_factor)
    }
}

pub fn create_test_circuit<const I: usize>(
    rng: &mut StdRng,
    use_crossover: bool,
    tx_hash: BlsScalar,
) -> Result<ExecuteCircuit<I, (), HEIGHT, ARITY>, Error> {
    let inputs = I as u64;
    let outputs = 2;

    let mut tree = Tree::<(), HEIGHT, ARITY>::new();

    let mut circuit = ExecuteCircuit::new();

    circuit.set_tx_hash(tx_hash);

    let mut transparent = false;

    let input_value = 100;
    let mut inputs_sum = 0;
    let mut input_data = vec![];

    let mut data_blocks = Vec::with_capacity(I);

    // Generate the notes and mutate the global tree state
    for pos in 0..inputs {
        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_spend_key();

        let mut note = create_test_note(rng, &psk, transparent, input_value);

        note.set_pos(pos);
        data_blocks.push(note);
        tree.insert(pos, WrappedNote(note));

        input_data.push((ssk, pos));

        transparent = !transparent;
        inputs_sum += input_value;
    }

    for (ssk, pos) in input_data.into_iter() {
        let note = data_blocks[pos as usize];
        let input = ExecuteCircuit::<I, (), HEIGHT, ARITY>::input(
            rng,
            &ssk,
            tx_hash,
            &tree,
            note.into(),
        )?;

        circuit.add_input(input)?;
    }

    let i = inputs as f64;
    let o = (outputs as f64).max(1.0);
    let output_value = (input_value as f64) * 0.7;
    let output_value = output_value * i / o;
    let output_value = output_value as u64;

    let mut outputs_sum = 0;
    for _ in 0..outputs {
        let ssk = SecretSpendKey::random(rng);
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();

        let note = create_test_note(rng, &psk, transparent, output_value);

        circuit.add_output(note, Some(&vk))?;

        transparent = !transparent;
        outputs_sum += output_value;
    }

    let ssk = SecretSpendKey::random(rng);
    let psk = ssk.public_spend_key();
    let value = inputs_sum - outputs_sum - 5;
    let blinding_factor = JubJubScalar::random(rng);
    let note = Note::obfuscated(rng, &psk, value, blinding_factor);

    let (mut fee, crossover) = note.try_into()?;
    fee.gas_price = 1;
    if use_crossover {
        fee.gas_limit = 5;
        circuit.set_fee_crossover(&fee, &crossover, value, blinding_factor);
    } else {
        fee.gas_limit = 5 + value;
        circuit.set_fee(&fee);
    }

    Ok(circuit)
}

#[test]
fn execute_1_2() {
    let rng = &mut StdRng::seed_from_u64(424242u64);

    let tx_hash = BlsScalar::random(rng);
    for use_crossover in [true, false].iter() {
        let circuit: ExecuteCircuitOneTwo =
            create_test_circuit::<1>(rng, *use_crossover, tx_hash)
                .expect("test circuit creation should pass");
        let (prover, verifier) = load_keys("ExecuteCircuitOneTwo")
            .expect("loading the keys should succeed");

        let (proof, pi) = prover
            .prove(rng, &circuit)
            .expect("creating a proof should succeed");

        verifier
            .verify(&proof, &pi)
            .expect("Proof verification should be successful");
    }
}

#[test]
fn execute_2_2() {
    let rng = &mut StdRng::seed_from_u64(424242u64);

    let tx_hash = BlsScalar::random(rng);
    for use_crossover in [true, false].iter() {
        let circuit: ExecuteCircuitTwoTwo =
            create_test_circuit::<2>(rng, *use_crossover, tx_hash)
                .expect("test circuit creation should pass");
        let (prover, verifier) = load_keys("ExecuteCircuitTwoTwo")
            .expect("loading the keys should succeed");

        let (proof, pi) = prover
            .prove(rng, &circuit)
            .expect("creating a proof should succeed");

        verifier
            .verify(&proof, &pi)
            .expect("Proof verification should be successful");
    }
}

#[test]
fn execute_3_2() {
    let rng = &mut StdRng::seed_from_u64(424242u64);

    let tx_hash = BlsScalar::random(rng);
    for use_crossover in [true, false].iter() {
        let circuit: ExecuteCircuitThreeTwo =
            create_test_circuit::<3>(rng, *use_crossover, tx_hash)
                .expect("test circuit creation should pass");
        let (prover, verifier) = load_keys("ExecuteCircuitThreeTwo")
            .expect("loading the keys should succeed");

        let (proof, pi) = prover
            .prove(rng, &circuit)
            .expect("creating a proof should succeed");

        verifier
            .verify(&proof, &pi)
            .expect("Proof verification should be successful");
    }
}

#[test]
fn execute_4_2() {
    let rng = &mut StdRng::seed_from_u64(424242u64);

    let tx_hash = BlsScalar::random(rng);
    for use_crossover in [true, false].iter() {
        let circuit: ExecuteCircuitFourTwo =
            create_test_circuit::<4>(rng, *use_crossover, tx_hash)
                .expect("test circuit creation should pass");
        let (prover, verifier) = load_keys("ExecuteCircuitFourTwo")
            .expect("loading the keys should succeed");

        let (proof, pi) = prover
            .prove(rng, &circuit)
            .expect("creating a proof should succeed");

        verifier
            .verify(&proof, &pi)
            .expect("Proof verification should be successful");
    }
}
