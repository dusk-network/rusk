// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{POSEIDON_TREE_DEPTH, TRANSCRIPT_LABEL};
use crate::error::Error;

use dusk_jubjub::GENERATOR_NUMS_EXTENDED;
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_poseidon::tree::{
    PoseidonBranch, PoseidonLeaf, PoseidonTree, PoseidonTreeAnnotation,
};
use dusk_poseidon::Error as PoseidonError;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;
mod variants;

pub use crossover::{CircuitCrossover, WitnessCrossover};
pub use input::{CircuitInput, CircuitInputSignature, WitnessInput};
pub use output::{CircuitOutput, WitnessOutput};

pub use variants::*;

#[cfg(feature = "builder")]
pub mod builder;

/// The circuit responsible for creating a zero-knowledge proof
#[derive(Debug, Clone)]
pub enum ExecuteCircuit {
    ExecuteCircuitOneTwo(ExecuteCircuitOneTwo),
    ExecuteCircuitTwoTwo(ExecuteCircuitTwoTwo),
    ExecuteCircuitThreeTwo(ExecuteCircuitThreeTwo),
    ExecuteCircuitFourTwo(ExecuteCircuitFourTwo),
}

impl Default for ExecuteCircuit {
    fn default() -> Self {
        Self::ExecuteCircuitOneTwo(Default::default())
    }
}

impl ExecuteCircuit {
    // Constant for a pedersen commitment with zero value.
    //
    // Calculated as `G^0 · G'^0`
    pub const ZERO_COMMITMENT: JubJubExtended =
        JubJubExtended::from_raw_unchecked(
            BlsScalar::zero(),
            BlsScalar::one(),
            BlsScalar::one(),
            BlsScalar::zero(),
            BlsScalar::one(),
        );

    pub fn input_signature<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        note: &Note,
        tx_hash: BlsScalar,
    ) -> CircuitInputSignature {
        CircuitInputSignature::sign(rng, ssk, note, tx_hash)
    }

    pub fn input_commitment(
        vk: &ViewKey,
        note: &Note,
    ) -> Result<(u64, JubJubScalar), Error> {
        let value = note.value(Some(vk))?;
        let blinding_factor = note.blinding_factor(Some(vk))?;

        Ok((value, blinding_factor))
    }

    pub fn input_branch<L, A>(
        tree: &PoseidonTree<L, A, { POSEIDON_TREE_DEPTH }>,
        pos: u64,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Error>
    where
        L: PoseidonLeaf + Into<Note>,
        A: PoseidonTreeAnnotation<L>,
    {
        Ok(tree.branch(pos)?.ok_or(PoseidonError::TreeBranchFailed)?)
    }

    pub fn input<R, L, A>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        tx_hash: BlsScalar,
        tree: &PoseidonTree<L, A, { POSEIDON_TREE_DEPTH }>,
        note: Note,
    ) -> Result<CircuitInput, Error>
    where
        R: RngCore + CryptoRng,
        L: PoseidonLeaf + Into<Note>,
        A: PoseidonTreeAnnotation<L>,
    {
        let signature = Self::input_signature(rng, ssk, &note, tx_hash);
        let nullifier = note.gen_nullifier(ssk);

        let stealth_address = note.stealth_address();
        let sk_r = ssk.sk_r(stealth_address);
        let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();
        let pk_r_p = pk_r_p.into();

        let vk = ssk.view_key();
        let (value, blinding_factor) = Self::input_commitment(&vk, &note)?;

        let pos = *note.pos();
        let branch = Self::input_branch(tree, pos)?;

        let input = CircuitInput::new(
            branch,
            note,
            pk_r_p,
            value,
            blinding_factor,
            nullifier,
            signature,
        );

        Ok(input)
    }

    pub fn add_input(&mut self, input: CircuitInput) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneTwo(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                if !c.inputs().is_empty() {
                    let mut c = ExecuteCircuitTwoTwo::new(
                        inputs, crossover, outputs, tx_hash,
                    );

                    c.add_input(input);
                    *self = Self::ExecuteCircuitTwoTwo(c);
                } else {
                    let mut c = ExecuteCircuitOneTwo::new(
                        inputs, crossover, outputs, tx_hash,
                    );

                    c.add_input(input);
                    *self = Self::ExecuteCircuitOneTwo(c);
                }
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );

                c.add_input(input);
                *self = Self::ExecuteCircuitThreeTwo(c);
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );

                c.add_input(input);
                *self = Self::ExecuteCircuitFourTwo(c);
            }
            _ => return Err(Error::CircuitMaximumInputs),
        }

        Ok(())
    }

    pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitTwoTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitThreeTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitFourTwo(c) => c.set_tx_hash(tx_hash),
        }
    }

    pub fn set_fee(&mut self, fee: &Fee) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitTwoTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitThreeTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitFourTwo(c) => c.set_fee(fee),
        }
    }

    pub fn set_fee_crossover(
        &mut self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
    ) {
        match self {
            Self::ExecuteCircuitOneTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            Self::ExecuteCircuitFourTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
        }
    }

    pub fn add_output_with_data(
        &mut self,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
    ) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneTwo(c) if c.outputs().len() < 2 => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            Self::ExecuteCircuitTwoTwo(c) if c.outputs().len() < 2 => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            Self::ExecuteCircuitThreeTwo(c) if c.outputs().len() < 2 => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            Self::ExecuteCircuitFourTwo(c) if c.outputs().len() < 2 => {
                c.add_output_with_data(note, value, blinding_factor)
            }

            _ => return Err(Error::CircuitMaximumOutputs),
        }

        Ok(())
    }

    pub fn add_output(
        &mut self,
        note: Note,
        vk: Option<&ViewKey>,
    ) -> Result<(), Error> {
        let value = note.value(vk)?;
        let blinding_factor = note.blinding_factor(vk)?;

        self.add_output_with_data(note, value, blinding_factor)
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitFourTwo(c) => c.public_inputs(),
        }
    }

    pub fn inputs(&self) -> &[CircuitInput] {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.inputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.inputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.inputs(),
            Self::ExecuteCircuitFourTwo(c) => c.inputs(),
        }
    }

    pub fn outputs(&self) -> &[CircuitOutput] {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.outputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.outputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.outputs(),
            Self::ExecuteCircuitFourTwo(c) => c.outputs(),
        }
    }

    pub const fn circuit_id(&self) -> &[u8; 32] {
        match self {
            Self::ExecuteCircuitOneTwo(_) => &ExecuteCircuitOneTwo::CIRCUIT_ID,
            Self::ExecuteCircuitTwoTwo(_) => &ExecuteCircuitTwoTwo::CIRCUIT_ID,
            Self::ExecuteCircuitThreeTwo(_) => {
                &ExecuteCircuitThreeTwo::CIRCUIT_ID
            }
            Self::ExecuteCircuitFourTwo(_) => {
                &ExecuteCircuitFourTwo::CIRCUIT_ID
            }
        }
    }

    pub fn compile(
        &mut self,
        pp: &PublicParameters,
    ) -> Result<(ProverKey, VerifierData), Error> {
        self.pad();

        match self {
            Self::ExecuteCircuitOneTwo(c) => Ok(c.compile(pp)?),
            Self::ExecuteCircuitTwoTwo(c) => Ok(c.compile(pp)?),
            Self::ExecuteCircuitThreeTwo(c) => Ok(c.compile(pp)?),
            Self::ExecuteCircuitFourTwo(c) => Ok(c.compile(pp)?),
        }
    }

    pub fn pad(&mut self) {
        match self {
            Self::ExecuteCircuitOneTwo(c) => c.pad(),
            Self::ExecuteCircuitTwoTwo(c) => c.pad(),
            Self::ExecuteCircuitThreeTwo(c) => c.pad(),
            Self::ExecuteCircuitFourTwo(c) => c.pad(),
        }
    }

    pub fn prove(
        &mut self,
        pp: &PublicParameters,
        pk: &ProverKey,
    ) -> Result<Proof, Error> {
        self.pad();

        match self {
            Self::ExecuteCircuitOneTwo(c) => {
                Ok(c.prove(pp, pk, TRANSCRIPT_LABEL)?)
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                Ok(c.prove(pp, pk, TRANSCRIPT_LABEL)?)
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                Ok(c.prove(pp, pk, TRANSCRIPT_LABEL)?)
            }
            Self::ExecuteCircuitFourTwo(c) => {
                Ok(c.prove(pp, pk, TRANSCRIPT_LABEL)?)
            }
        }
    }

    pub fn verify(
        pp: &PublicParameters,
        vd: &VerifierData,
        proof: &Proof,
        public_inputs: &[PublicInputValue],
    ) -> Result<(), Error> {
        // Since we take the verifier data as parameter, we can use any of the
        // variants
        Ok(ExecuteCircuitTwoTwo::verify(
            pp,
            vd,
            proof,
            public_inputs,
            TRANSCRIPT_LABEL,
        )?)
    }
}
