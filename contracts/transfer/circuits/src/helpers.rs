// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ExecuteCircuit;

use anyhow::{anyhow, Result};
use canonical::Store;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
use phoenix_core::Note;
use poseidon252::tree::{PoseidonAnnotation, PoseidonTree};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;
use std::convert::TryInto;

mod leaf;

pub use leaf::NoteLeaf;

impl<const DEPTH: usize, const CAPACITY: usize>
    ExecuteCircuit<DEPTH, CAPACITY>
{
    pub fn create_dummy_note<R: RngCore + CryptoRng>(
        rng: &mut R,
        psk: &PublicSpendKey,
        transparent: bool,
        value: u64,
    ) -> (Note, JubJubScalar) {
        if transparent {
            let note = Note::transparent(rng, &psk, value);
            let blinding_factor =
                note.blinding_factor(None).unwrap_or_default();

            (note, blinding_factor)
        } else {
            let blinding_factor = JubJubScalar::random(rng);
            let note = Note::obfuscated(rng, &psk, value, blinding_factor);

            (note, blinding_factor)
        }
    }

    pub fn create_dummy_circuit<R: RngCore + CryptoRng, S: Store>(
        rng: &mut R,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self> {
        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, S, DEPTH>::new();

        let mut circuit = ExecuteCircuit::<DEPTH, CAPACITY>::default();

        let mut transparent = false;

        let input_value = 100;
        let mut inputs_sum = 0;
        for _ in 0..inputs {
            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_key();

            let (note, blinding_factor) =
                Self::create_dummy_note(rng, &psk, transparent, input_value);

            let pos = tree.push(note.into()).map_err(|e| {
                anyhow!("Failed append note to the tree: {}", e)
            })?;

            let note = tree
                .get(pos)
                .map_err(|e| {
                    anyhow!("Failed to fetch note from the tree: {}", e)
                })?
                .map(|n| Note::from(n))
                .ok_or(anyhow!("Note not found in the tree after push!"))?;

            let branch = tree
                .branch(pos)
                .map_err(|e| anyhow!("Failed to get the branch: {}", e))?
                .ok_or(anyhow!("Failed to fetch the branch from the tree"))?;

            let sk_r = ssk.sk_r(note.stealth_address());
            let nullifier = note.gen_nullifier(&ssk);
            circuit.add_input(
                rng,
                branch,
                sk_r,
                note,
                input_value,
                blinding_factor,
                nullifier,
            )?;

            transparent = !transparent;
            inputs_sum += input_value;
        }

        let i = inputs as f64;
        let o = (outputs as f64).max(1.0);
        let output_value = (input_value as f64) * 0.7;
        let output_value = output_value * i / o;
        let output_value = output_value as u64;

        let mut outputs_sum = 0;
        for _ in 0..outputs {
            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_key();

            let (note, blinding_factor) =
                Self::create_dummy_note(rng, &psk, transparent, output_value);

            circuit.add_output(note, output_value, blinding_factor);

            transparent = !transparent;
            outputs_sum += output_value;
        }

        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_key();
        let value = inputs_sum - outputs_sum;
        let blinding_factor = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, &psk, value, blinding_factor);
        let (_, crossover) = note.try_into().map_err(|e| {
            anyhow!(
                "Failed to generate crossover from obfuscated note: {:?}",
                e
            )
        })?;
        let value_commitment = *crossover.value_commitment();
        circuit.set_crossover(value_commitment, value, blinding_factor);

        Ok(circuit)
    }

    pub fn create_dummy_proof<R: RngCore + CryptoRng, S: Store>(
        rng: &mut R,
        rusk_profile: bool,
        inputs: usize,
        outputs: usize,
    ) -> Result<(
        Self,
        PublicParameters,
        ProverKey,
        VerifierKey,
        Proof,
        Vec<PublicInput>,
    )> {
        let mut circuit =
            Self::create_dummy_circuit::<R, S>(rng, inputs, outputs)?;

        let (pp, pk, vk) = if rusk_profile {
            // Verifier key from Rusk Profile is corrupted
            // https://github.com/dusk-network/rusk/issues/159
            let (pp, pk, _) = circuit.rusk_circuit_args()?;
            let (_, vk) = circuit.compile(&pp)?;

            circuit.get_mut_pi_positions().clear();

            (pp, pk, vk)
        } else {
            let pp = PublicParameters::setup(circuit.get_trim_size(), rng)?;
            let (pk, vk) = circuit.compile(&pp)?;

            (pp, pk, vk)
        };

        let label = circuit.transcript_label();
        let proof = circuit.gen_proof(&pp, &pk, label)?;
        let pi = circuit.get_pi_positions().clone();

        Ok((circuit, pp, pk, vk, proof, pi))
    }
}
