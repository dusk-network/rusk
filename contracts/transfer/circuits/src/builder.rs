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
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

impl<const DEPTH: usize, const CAPACITY: usize>
    ExecuteCircuit<DEPTH, CAPACITY>
{
    pub const fn transcript_label() -> &'static [u8] {
        b"execute-circuit"
    }

    pub fn create_dummy_note<R: RngCore + CryptoRng>(
        rng: &mut R,
        psk: &PublicSpendKey,
        transparent: bool,
        value: u64,
    ) -> Note {
        if transparent {
            Note::transparent(rng, &psk, value)
        } else {
            let blinding_factor = JubJubScalar::random(rng);

            Note::obfuscated(rng, &psk, value, blinding_factor)
        }
    }

    pub fn create_dummy_circuit<R: RngCore + CryptoRng, S: Store>(
        rng: &mut R,
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
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

            let note =
                Self::create_dummy_note(rng, &psk, transparent, input_value);

            let pos = tree.push(note.into()).map_err(|e| {
                anyhow!("Failed append note to the tree: {}", e)
            })?;

            let note = tree
                .get(pos as usize)
                .map_err(|e| anyhow!("Internal poseidon tree error: {:?}", e))?
                .map(|n| n.into())
                .ok_or(anyhow!("Inserted note not found in the tree!"))?;
            let signature =
                ExecuteCircuit::<DEPTH, CAPACITY>::sign(rng, &ssk, &note);

            input_data.push((ssk, pos, signature));

            transparent = !transparent;
            inputs_sum += input_value;
        }

        for (ssk, pos, signature) in input_data.into_iter() {
            circuit.add_input(&ssk, &tree, pos, signature)?;
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
        let vk = ssk.view_key();
        let psk = ssk.public_spend_key();
        let value = inputs_sum - outputs_sum - 5;
        let blinding_factor = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, &psk, value, blinding_factor);
        let (mut fee, crossover) = note.try_into().map_err(|e| {
            anyhow!(
                "Failed to generate crossover from obfuscated note: {:?}",
                e
            )
        })?;
        if use_crossover {
            fee.gas_limit = 5;
            circuit.set_fee_crossover(&fee, &crossover, &vk)?;
        } else {
            fee.gas_limit = 5 + value;
            circuit.set_fee(&fee)?;
        }

        Ok(circuit)
    }

    pub fn create_dummy_proof<R: RngCore + CryptoRng, S: Store>(
        rng: &mut R,
        pp: Option<PublicParameters>,
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
    ) -> Result<(
        Self,
        PublicParameters,
        ProverKey,
        VerifierKey,
        Proof,
        Vec<PublicInput>,
    )> {
        let mut circuit = Self::create_dummy_circuit::<R, S>(
            rng,
            inputs,
            outputs,
            use_crossover,
        )?;

        let id = circuit.rusk_keys_id();
        let (pp, pk, vk) = circuit_keys(rng, pp, &mut circuit, id)?;

        let label = Self::transcript_label();
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
        let (pk, vk) = keys.get(id).ok_or(anyhow!(
            "Failed to get '{}' keys for '{}' from Rusk profile",
            id,
            env!("CARGO_PKG_NAME")
        ))?;

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
