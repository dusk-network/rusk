// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::ExecuteCircuit;
use crate::error::Error;
use crate::POSEIDON_TREE_DEPTH;

use canonical_derive::Canon;
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use dusk_poseidon::Error as PoseidonError;
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

#[derive(Debug, Clone, Canon)]
pub struct NoteLeaf(Note);

impl AsRef<Note> for NoteLeaf {
    fn as_ref(&self) -> &Note {
        &self.0
    }
}

impl From<Note> for NoteLeaf {
    fn from(note: Note) -> NoteLeaf {
        NoteLeaf(note)
    }
}

impl From<NoteLeaf> for Note {
    fn from(leaf: NoteLeaf) -> Note {
        leaf.0
    }
}

impl PoseidonLeaf for NoteLeaf {
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    fn pos(&self) -> &u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos)
    }
}

impl ExecuteCircuit {
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
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
        tx_hash: BlsScalar,
    ) -> Result<Self, Error> {
        let mut tree = PoseidonTree::<
            NoteLeaf,
            PoseidonAnnotation,
            POSEIDON_TREE_DEPTH,
        >::new();

        let mut circuit = ExecuteCircuit::default();

        circuit.set_tx_hash(tx_hash);

        let mut transparent = false;

        let input_value = 100;
        let mut inputs_sum = 0;
        let mut input_data = vec![];

        // Generate the notes and mutate the global tree state
        for _ in 0..inputs {
            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_spend_key();

            let note =
                Self::create_dummy_note(rng, &psk, transparent, input_value);

            let pos = tree.push(note.into())?;

            input_data.push((ssk, pos));

            transparent = !transparent;
            inputs_sum += input_value;
        }

        for (ssk, pos) in input_data.into_iter() {
            let note = tree.get(pos)?.ok_or(PoseidonError::TreeGetFailed)?;
            let input = Self::input(rng, &ssk, tx_hash, &tree, note.into())?;

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

            let note =
                Self::create_dummy_note(rng, &psk, transparent, output_value);

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
            circuit.set_fee(&fee)?;
        }

        Ok(circuit)
    }

    #[allow(clippy::type_complexity)]
    pub fn create_dummy_proof<R: RngCore + CryptoRng>(
        rng: &mut R,
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
        tx_hash: BlsScalar,
    ) -> Result<
        (
            Self,
            PublicParameters,
            ProverKey,
            VerifierData,
            Proof,
            Vec<PublicInputValue>,
        ),
        Error,
    > {
        let mut execute = Self::create_dummy_circuit::<R>(
            rng,
            inputs,
            outputs,
            use_crossover,
            tx_hash,
        )?;

        let pi = execute.public_inputs();

        let pp =
            rusk_profile::get_common_reference_string().map(|pp| unsafe {
                PublicParameters::from_slice_unchecked(pp.as_slice())
            })?;

        let keys = rusk_profile::keys_for(execute.circuit_id())?;
        let pk = keys.get_prover()?;
        let vd = keys.get_verifier()?;

        let pk = ProverKey::from_slice(pk.as_slice())?;
        let vd = VerifierData::from_slice(vd.as_slice())?;

        let proof = execute.prove(&pp, &pk)?;

        Ok((execute, pp, pk, vd, proof, pi))
    }
}
