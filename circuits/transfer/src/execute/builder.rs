// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ExecuteCircuit;
use std::convert::TryInto;
use std::env;

use anyhow::{anyhow, Result};
use canonical_derive::Canon;
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::circuit::VerifierData;
use dusk_poseidon::tree::{PoseidonAnnotation, PoseidonLeaf, PoseidonTree};
use phoenix_core::Note;
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

const POSEIDON_BRANCH_DEPTH: usize = 17;

pub fn circuit_keys<R, C>(
    rng: &mut R,
    pp: Option<PublicParameters>,
    circuit: &mut C,
    id: &str,
    use_rusk_profile: bool,
) -> Result<(PublicParameters, ProverKey, VerifierData)>
where
    R: RngCore + CryptoRng,
    C: Circuit,
{
    let use_rusk_profile = match env::var("RUSK_PROFILE_KEYS") {
        Ok(p) => p.parse()?,
        _ => use_rusk_profile,
    };

    if use_rusk_profile {
        let pp = pp.map(|pp| Ok(pp)).unwrap_or({
            rusk_profile::get_common_reference_string()
                .map_err(|e| {
                    anyhow!("Failed to fetch CRS from rusk profile: {}", e)
                })
                .map(|pp| unsafe {
                    PublicParameters::from_slice_unchecked(pp.as_slice())
                })
        })?;

        let keys = rusk_profile::keys_for(env!("CARGO_PKG_NAME"));
        let (pk, vd) = keys.get(id).ok_or(anyhow!(
            "Failed to get '{}' keys for '{}' from Rusk profile",
            id,
            env!("CARGO_PKG_NAME")
        ))?;

        let pk = ProverKey::from_slice(pk.as_slice())?;
        let vd = VerifierData::from_slice(vd.as_slice())?;

        Ok((pp, pk, vd))
    } else {
        let pp = pp.map(|pp| Ok(pp)).unwrap_or(PublicParameters::setup(
            circuit.padded_circuit_size(),
            rng,
        ))?;

        let (pk, vd) = circuit.compile(&pp)?;

        Ok((pp, pk, vd))
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
            Note::transparent(rng, &psk, value)
        } else {
            let blinding_factor = JubJubScalar::random(rng);

            Note::obfuscated(rng, &psk, value, blinding_factor)
        }
    }

    pub fn create_dummy_circuit<R: RngCore + CryptoRng>(
        rng: &mut R,
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
    ) -> Result<Self> {
        let mut tree = PoseidonTree::<
            NoteLeaf,
            PoseidonAnnotation,
            POSEIDON_BRANCH_DEPTH,
        >::new();

        let mut circuit = ExecuteCircuit::default();

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
                .get(pos)
                .map_err(|e| anyhow!("Internal poseidon tree error: {:?}", e))?
                .map(|n| n.into())
                .ok_or(anyhow!("Inserted note not found in the tree!"))?;
            let signature = ExecuteCircuit::sign(rng, &ssk, &note);

            input_data.push((ssk, pos, signature));

            transparent = !transparent;
            inputs_sum += input_value;
        }

        for (ssk, pos, signature) in input_data.into_iter() {
            circuit.add_input_from_tree(&ssk, &tree, pos, signature)?;
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
        fee.gas_price = 1;
        if use_crossover {
            fee.gas_limit = 5;
            circuit.set_fee_crossover(&fee, &crossover, &vk)?;
        } else {
            fee.gas_limit = 5 + value;
            circuit.set_fee(&fee)?;
        }

        Ok(circuit)
    }

    pub fn create_dummy_proof<R: RngCore + CryptoRng>(
        rng: &mut R,
        pp: Option<PublicParameters>,
        inputs: usize,
        outputs: usize,
        use_crossover: bool,
        use_rusk_profile: bool,
    ) -> Result<(
        Self,
        PublicParameters,
        ProverKey,
        VerifierData,
        Proof,
        Vec<PublicInputValue>,
    )> {
        let mut circuit = Self::create_dummy_circuit::<R>(
            rng,
            inputs,
            outputs,
            use_crossover,
        )?;

        let id = circuit.rusk_keys_id();
        let (pp, pk, vd) =
            circuit_keys(rng, pp, &mut circuit, id, use_rusk_profile)?;

        // FIXME Repeated definition of circuit label
        let label = b"dusk-network";
        let proof = circuit.gen_proof(&pp, &pk, label)?;
        let pi = circuit.public_inputs();

        Ok((circuit, pp, pk, vd, proof, pi))
    }
}
