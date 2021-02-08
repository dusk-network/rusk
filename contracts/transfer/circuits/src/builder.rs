// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ExecuteCircuit;
use std::convert::TryInto;

use anyhow::{anyhow, Result};
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
use phoenix_core::Note;
use poseidon252::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

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
        let mut input_data = vec![];

        // Generate the notes and mutate the global tree state
        for _ in 0..inputs {
            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_spend_key();

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

            let sk_r = ssk.sk_r(note.stealth_address()).as_ref().clone();
            let nullifier = note.gen_nullifier(&ssk);

            input_data.push((
                sk_r,
                note,
                input_value,
                blinding_factor,
                nullifier,
            ));

            transparent = !transparent;
            inputs_sum += input_value;
        }

        for (sk_r, note, input_value, blinding_factor, nullifier) in
            input_data.into_iter()
        {
            let branch = tree
                .branch(note.pos() as usize)
                .map_err(|e| anyhow!("Failed to get the branch: {}", e))?
                .ok_or(anyhow!("Failed to fetch the branch from the tree"))?;

            circuit.add_input(
                rng,
                branch,
                sk_r,
                note,
                input_value,
                blinding_factor,
                nullifier,
            )?;
        }

        let i = inputs as f64;
        let o = (outputs as f64).max(1.0);
        let output_value = (input_value as f64) * 0.7;
        let output_value = output_value * i / o;
        let output_value = output_value as u64;

        let mut outputs_sum = 0;
        for _ in 0..outputs {
            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_spend_key();

            let (note, blinding_factor) =
                Self::create_dummy_note(rng, &psk, transparent, output_value);

            circuit.add_output(note, output_value, blinding_factor);

            transparent = !transparent;
            outputs_sum += output_value;
        }

        let ssk = SecretSpendKey::random(rng);
        let psk = ssk.public_spend_key();
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
        pp: Option<PublicParameters>,
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

        let id = circuit.rusk_keys_id();
        let (pp, pk, vk) = circuit_keys(rng, pp, &mut circuit, id.as_str())?;

        let label = circuit.transcript_label();
        let proof = circuit.gen_proof(&pp, &pk, label)?;
        let mut pi = circuit.get_pi_positions().clone();

        // Reset PI positions to emulate real-world verification
        pi.iter_mut().for_each(|p| match p {
            PublicInput::BlsScalar(_, p) => *p = 0,
            PublicInput::JubJubScalar(_, p) => *p = 0,
            PublicInput::AffinePoint(_, p, q) => {
                *p = 0;
                *q = 0;
            }
        });

        Ok((circuit, pp, pk, vk, proof, pi))
    }
}

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

impl<S> PoseidonLeaf<S> for NoteLeaf
where
    S: Store,
{
    fn poseidon_hash(&self) -> BlsScalar {
        self.0.hash()
    }

    fn pos(&self) -> u64 {
        self.0.pos()
    }

    fn set_pos(&mut self, pos: u64) {
        self.0.set_pos(pos)
    }
}

#[allow(unused_variables)]
pub fn circuit_keys<'a, R, C>(
    rng: &mut R,
    pp: Option<PublicParameters>,
    circuit: &mut C,
    id: &str,
) -> Result<(PublicParameters, ProverKey, VerifierKey)>
where
    R: RngCore + CryptoRng,
    C: Circuit<'a>,
{
    #[cfg(not(feature = "builder-no-rusk-profile-keys"))]
    {
        let pp = pp.map(|pp| Ok(pp)).unwrap_or({
            rusk_profile::get_common_reference_string()
                .map_err(|e| {
                    anyhow!("Failed to fetch CRS from rusk profile: {}", e)
                })
                .and_then(|pp| unsafe {
                    PublicParameters::from_slice_unchecked(pp.as_slice())
                })
        })?;

        let keys = rusk_profile::keys_for(env!("CARGO_PKG_NAME"));
        let (pk, vk) = keys
            .get(id)
            .ok_or(anyhow!("Failed to get keys from Rusk profile"))?;

        let pk = ProverKey::from_bytes(pk.as_slice())?;
        let vk = VerifierKey::from_bytes(vk.as_slice())?;

        Ok((pp, pk, vk))
    }

    #[cfg(feature = "builder-no-rusk-profile-keys")]
    {
        let pp = pp
            .map(|pp| Ok(pp))
            .unwrap_or(PublicParameters::setup(circuit.get_trim_size(), rng))?;

        let (pk, vk) = circuit.compile(&pp)?;
        circuit.get_mut_pi_positions().clear();

        Ok((pp, pk, vk))
    }
}
