// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{
    ExecuteCircuitFourTwo, ExecuteCircuitOneTwo, ExecuteCircuitThreeTwo,
    ExecuteCircuitTwoTwo,
};

use crate::error::Error;
use crate::execute::ExecuteCircuit;

use dusk_merkle::Aggregate;
use poseidon_merkle::{Item, Tree};

use dusk_pki::{PublicSpendKey, SecretSpendKey};
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

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

macro_rules! execute_circuit_variant {
    ($ty:ident<T, H, A>, $i:literal, $o:literal) => {
        impl<T, const H: usize, const A: usize> $ty<T, H, A> {
            pub fn create_dummy_note<R: RngCore + CryptoRng>(
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

            pub fn create_dummy_circuit<R: RngCore + CryptoRng>(
                rng: &mut R,
                use_crossover: bool,
                tx_hash: BlsScalar,
            ) -> Result<Self, Error>
            where
                T: Clone + Default + Aggregate<A>,
            {
                let inputs = $i;
                let outputs = $o;

                let mut tree = Tree::<T, H, A>::new();

                let mut circuit = Self::default();

                circuit.set_tx_hash(tx_hash);

                let mut transparent = false;

                let input_value = 100;
                let mut inputs_sum = 0;
                let mut input_data = vec![];

                let mut data_blocks = Vec::with_capacity($i);

                // Generate the notes and mutate the global tree state
                for pos in 0..inputs {
                    let ssk = SecretSpendKey::random(rng);
                    let psk = ssk.public_spend_key();

                    let mut note = Self::create_dummy_note(
                        rng,
                        &psk,
                        transparent,
                        input_value,
                    );

                    note.set_pos(pos);
                    data_blocks.push(note);
                    tree.insert(pos, WrappedNote(note));

                    input_data.push((ssk, pos));

                    transparent = !transparent;
                    inputs_sum += input_value;
                }

                for (ssk, pos) in input_data.into_iter() {
                    let note = data_blocks[pos as usize];
                    let input = ExecuteCircuit::input(
                        rng,
                        &ssk,
                        tx_hash,
                        &tree,
                        note.into(),
                    )?;

                    circuit.add_input(input);
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

                    let note = Self::create_dummy_note(
                        rng,
                        &psk,
                        transparent,
                        output_value,
                    );

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
                    circuit.set_fee_crossover(
                        &fee,
                        &crossover,
                        value,
                        blinding_factor,
                    );
                } else {
                    fee.gas_limit = 5 + value;
                    circuit.set_fee(&fee);
                }

                Ok(circuit)
            }

            #[allow(clippy::type_complexity)]
            pub fn create_dummy_proof<R: RngCore + CryptoRng>(
                rng: &mut R,
                use_crossover: bool,
                tx_hash: BlsScalar,
            ) -> Result<(Self, Prover, Verifier, Proof, Vec<BlsScalar>), Error>
            where
                T: Clone + Default + Aggregate<A>,
            {
                let circuit = Self::create_dummy_circuit::<R>(
                    rng,
                    use_crossover,
                    tx_hash,
                )?;

                let keys = rusk_profile::keys_for(Self::circuit_id())?;
                let pk = keys.get_prover()?;
                let vd = keys.get_verifier()?;

                let prover = Prover::try_from_bytes(pk.as_slice())?;
                let verifier = Verifier::try_from_bytes(vd.as_slice())?;

                let (proof, pi) = prover.prove(rng, &circuit)?;

                Ok((circuit, prover, verifier, proof, pi))
            }
        }
    };
}

execute_circuit_variant!(ExecuteCircuitOneTwo<T, H, A>, 1, 2);
execute_circuit_variant!(ExecuteCircuitTwoTwo<T, H, A>, 2, 2);
execute_circuit_variant!(ExecuteCircuitThreeTwo<T, H, A>, 3, 2);
execute_circuit_variant!(ExecuteCircuitFourTwo<T, H, A>, 4, 2);
