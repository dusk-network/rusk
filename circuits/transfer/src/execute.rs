// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;

use crossover::CircuitCrossover;
use input::{CircuitInput, WitnessInput};
use output::{CircuitOutput, WitnessOutput};

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubScalar;
use dusk_pki::{Ownable, SecretKey, SecretSpendKey, ViewKey};
use dusk_poseidon::tree::{
    PoseidonBranch, PoseidonLeaf, PoseidonTree, PoseidonTreeAnnotation,
};
use dusk_poseidon::Error as PoseidonError;
use dusk_schnorr::Proof as SchnorrProof;
use input::POSEIDON_BRANCH_DEPTH;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;
mod variants;

pub use variants::*;

#[cfg(any(test, feature = "builder"))]
pub mod builder;

/// Constant message for the schnorr signature generation
///
/// The signature is provided outside the circuit; so that's why it is
/// constant
///
/// The contents of the message are yet to be defined in the documentation.
/// For now, it is treated as a constant.
///
/// https://github.com/dusk-network/rusk/issues/178
pub(crate) const SIGN_MESSAGE: BlsScalar = BlsScalar::one();

/// The circuit responsible for creating a zero-knowledge proof
#[derive(Debug, Clone)]
pub enum ExecuteCircuit {
    ExecuteCircuitOneZero(ExecuteCircuitOneZero),
    ExecuteCircuitOneOne(ExecuteCircuitOneOne),
    ExecuteCircuitOneTwo(ExecuteCircuitOneTwo),
    ExecuteCircuitTwoZero(ExecuteCircuitTwoZero),
    ExecuteCircuitTwoOne(ExecuteCircuitTwoOne),
    ExecuteCircuitTwoTwo(ExecuteCircuitTwoTwo),
    ExecuteCircuitThreeZero(ExecuteCircuitThreeZero),
    ExecuteCircuitThreeOne(ExecuteCircuitThreeOne),
    ExecuteCircuitThreeTwo(ExecuteCircuitThreeTwo),
    ExecuteCircuitFourZero(ExecuteCircuitFourZero),
    ExecuteCircuitFourOne(ExecuteCircuitFourOne),
    ExecuteCircuitFourTwo(ExecuteCircuitFourTwo),
}

impl Default for ExecuteCircuit {
    fn default() -> Self {
        Self::ExecuteCircuitOneZero(Default::default())
    }
}

impl ExecuteCircuit {
    pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
        match self {
            Self::ExecuteCircuitOneZero(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitOneOne(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitOneTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitTwoZero(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitTwoOne(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitTwoTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitThreeZero(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitThreeOne(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitThreeTwo(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitFourZero(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitFourOne(c) => c.set_tx_hash(tx_hash),
            Self::ExecuteCircuitFourTwo(c) => c.set_tx_hash(tx_hash),
        }
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        note: &Note,
    ) -> SchnorrProof {
        let message = SIGN_MESSAGE;
        let sk_r = *ssk.sk_r(note.stealth_address()).as_ref();
        let secret = SecretKey::from(&sk_r);

        SchnorrProof::new(&secret, rng, message)
    }

    pub fn add_input(
        &mut self,
        ssk: &SecretSpendKey,
        note: Note,
        branch: PoseidonBranch<{ input::POSEIDON_BRANCH_DEPTH }>,
        signature: SchnorrProof,
    ) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneZero(c) => {
                let result;
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                if !c.inputs().is_empty() {
                    let mut c = ExecuteCircuitTwoZero::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitTwoZero(c);
                } else {
                    let mut c = ExecuteCircuitOneZero::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitOneZero(c);
                }
                result
            }
            Self::ExecuteCircuitOneOne(c) => {
                let result;
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                if !c.inputs().is_empty() {
                    let mut c = ExecuteCircuitTwoOne::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitTwoOne(c);
                } else {
                    let mut c = ExecuteCircuitOneOne::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitOneOne(c);
                }
                result
            }
            Self::ExecuteCircuitOneTwo(c) => {
                let result;
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                if !c.inputs().is_empty() {
                    let mut c = ExecuteCircuitTwoTwo::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitTwoTwo(c);
                } else {
                    let mut c = ExecuteCircuitOneTwo::new(
                        inputs, crossover, outputs, tx_hash,
                    );
                    result = c.add_input(ssk, note, branch, signature);
                    *self = Self::ExecuteCircuitOneTwo(c);
                }
                result
            }
            Self::ExecuteCircuitTwoZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeZero::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitThreeZero(c);
                result
            }
            Self::ExecuteCircuitTwoOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitThreeOne(c);
                result
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitThreeTwo(c);
                result
            }
            Self::ExecuteCircuitThreeZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourZero::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitFourZero(c);
                result
            }
            Self::ExecuteCircuitThreeOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitFourOne(c);
                result
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                let result = c.add_input(ssk, note, branch, signature);
                *self = Self::ExecuteCircuitFourTwo(c);
                result
            }
            _ => Err(Error::CircuitMaximumInputs),
        }
    }

    pub fn add_input_from_tree<L, A>(
        &mut self,
        ssk: &SecretSpendKey,
        tree: &PoseidonTree<L, A, { input::POSEIDON_BRANCH_DEPTH }>,
        pos: u64,
        signature: SchnorrProof,
    ) -> Result<(), Error>
    where
        L: PoseidonLeaf + Into<Note>,
        A: PoseidonTreeAnnotation<L>,
    {
        let note = tree
            .get(pos)?
            .map(|n| n.into())
            .ok_or(PoseidonError::TreeGetFailed)?;

        let branch =
            tree.branch(pos)?.ok_or(PoseidonError::TreeBranchFailed)?;

        self.add_input(ssk, note, branch, signature)
    }

    pub fn set_fee(&mut self, fee: &Fee) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneZero(c) => c.set_fee(fee),
            Self::ExecuteCircuitOneOne(c) => c.set_fee(fee),
            Self::ExecuteCircuitOneTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitTwoZero(c) => c.set_fee(fee),
            Self::ExecuteCircuitTwoOne(c) => c.set_fee(fee),
            Self::ExecuteCircuitTwoTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitThreeZero(c) => c.set_fee(fee),
            Self::ExecuteCircuitThreeOne(c) => c.set_fee(fee),
            Self::ExecuteCircuitThreeTwo(c) => c.set_fee(fee),
            Self::ExecuteCircuitFourZero(c) => c.set_fee(fee),
            Self::ExecuteCircuitFourOne(c) => c.set_fee(fee),
            Self::ExecuteCircuitFourTwo(c) => c.set_fee(fee),
        }
    }

    pub fn set_fee_crossover(
        &mut self,
        fee: &Fee,
        crossover: &Crossover,
        vk: &ViewKey,
    ) -> Result<(), Error> {
        match self {
            Self::ExecuteCircuitOneZero(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitOneOne(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitOneTwo(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitTwoZero(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitTwoOne(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitThreeZero(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitThreeOne(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitFourZero(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitFourOne(c) => {
                c.set_fee_crossover(fee, crossover, vk)
            }
            Self::ExecuteCircuitFourTwo(c) => {
                c.set_fee_crossover(fee, crossover, vk)
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
            Self::ExecuteCircuitOneZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitOneOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitOneOne(c);
                Ok(())
            }

            Self::ExecuteCircuitOneOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitOneTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitOneTwo(c);
                Ok(())
            }

            Self::ExecuteCircuitTwoZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitTwoOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitTwoOne(c);
                Ok(())
            }

            Self::ExecuteCircuitTwoOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitTwoTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitTwoTwo(c);
                Ok(())
            }

            Self::ExecuteCircuitThreeZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitThreeOne(c);
                Ok(())
            }

            Self::ExecuteCircuitThreeOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitThreeTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitThreeTwo(c);
                Ok(())
            }

            Self::ExecuteCircuitFourZero(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourOne::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitFourOne(c);
                Ok(())
            }

            Self::ExecuteCircuitFourOne(c) => {
                let (inputs, crossover, outputs, tx_hash) = c.into_inner();
                let mut c = ExecuteCircuitFourTwo::new(
                    inputs, crossover, outputs, tx_hash,
                );
                c.add_output_with_data(note, value, blinding_factor);
                *self = Self::ExecuteCircuitFourTwo(c);
                Ok(())
            }

            _ => Err(Error::CircuitMaximumOutputs),
        }
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
            Self::ExecuteCircuitOneZero(c) => c.public_inputs(),
            Self::ExecuteCircuitOneOne(c) => c.public_inputs(),
            Self::ExecuteCircuitOneTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitTwoZero(c) => c.public_inputs(),
            Self::ExecuteCircuitTwoOne(c) => c.public_inputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitThreeZero(c) => c.public_inputs(),
            Self::ExecuteCircuitThreeOne(c) => c.public_inputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.public_inputs(),
            Self::ExecuteCircuitFourZero(c) => c.public_inputs(),
            Self::ExecuteCircuitFourOne(c) => c.public_inputs(),
            Self::ExecuteCircuitFourTwo(c) => c.public_inputs(),
        }
    }

    pub fn inputs(&self) -> &[CircuitInput] {
        match self {
            Self::ExecuteCircuitOneZero(c) => c.inputs(),
            Self::ExecuteCircuitOneOne(c) => c.inputs(),
            Self::ExecuteCircuitOneTwo(c) => c.inputs(),
            Self::ExecuteCircuitTwoZero(c) => c.inputs(),
            Self::ExecuteCircuitTwoOne(c) => c.inputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.inputs(),
            Self::ExecuteCircuitThreeZero(c) => c.inputs(),
            Self::ExecuteCircuitThreeOne(c) => c.inputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.inputs(),
            Self::ExecuteCircuitFourZero(c) => c.inputs(),
            Self::ExecuteCircuitFourOne(c) => c.inputs(),
            Self::ExecuteCircuitFourTwo(c) => c.inputs(),
        }
    }

    pub fn outputs(&self) -> &[CircuitOutput] {
        match self {
            Self::ExecuteCircuitOneZero(c) => c.outputs(),
            Self::ExecuteCircuitOneOne(c) => c.outputs(),
            Self::ExecuteCircuitOneTwo(c) => c.outputs(),
            Self::ExecuteCircuitTwoZero(c) => c.outputs(),
            Self::ExecuteCircuitTwoOne(c) => c.outputs(),
            Self::ExecuteCircuitTwoTwo(c) => c.outputs(),
            Self::ExecuteCircuitThreeZero(c) => c.outputs(),
            Self::ExecuteCircuitThreeOne(c) => c.outputs(),
            Self::ExecuteCircuitThreeTwo(c) => c.outputs(),
            Self::ExecuteCircuitFourZero(c) => c.outputs(),
            Self::ExecuteCircuitFourOne(c) => c.outputs(),
            Self::ExecuteCircuitFourTwo(c) => c.outputs(),
        }
    }

    pub const fn circuit_id(&self) -> &[u8; 32] {
        match self {
            Self::ExecuteCircuitOneZero(_) => {
                &ExecuteCircuitOneZero::CIRCUIT_ID
            }
            Self::ExecuteCircuitOneOne(_) => &ExecuteCircuitOneOne::CIRCUIT_ID,
            Self::ExecuteCircuitOneTwo(_) => &ExecuteCircuitOneTwo::CIRCUIT_ID,
            Self::ExecuteCircuitTwoZero(_) => {
                &ExecuteCircuitTwoZero::CIRCUIT_ID
            }
            Self::ExecuteCircuitTwoOne(_) => &ExecuteCircuitTwoOne::CIRCUIT_ID,
            Self::ExecuteCircuitTwoTwo(_) => &ExecuteCircuitTwoTwo::CIRCUIT_ID,
            Self::ExecuteCircuitThreeZero(_) => {
                &ExecuteCircuitThreeZero::CIRCUIT_ID
            }
            Self::ExecuteCircuitThreeOne(_) => {
                &ExecuteCircuitThreeOne::CIRCUIT_ID
            }
            Self::ExecuteCircuitThreeTwo(_) => {
                &ExecuteCircuitThreeTwo::CIRCUIT_ID
            }
            Self::ExecuteCircuitFourZero(_) => {
                &ExecuteCircuitFourZero::CIRCUIT_ID
            }
            Self::ExecuteCircuitFourOne(_) => {
                &ExecuteCircuitFourOne::CIRCUIT_ID
            }
            Self::ExecuteCircuitFourTwo(_) => {
                &ExecuteCircuitFourTwo::CIRCUIT_ID
            }
        }
    }

    /// Wrapper method required while circuit implementation is not object safe
    /// https://github.com/dusk-network/plonk/issues/531
    pub fn gen_proof(
        &mut self,
        pp: &PublicParameters,
        pk: &ProverKey,
        label: &'static [u8],
    ) -> Result<Proof, Error> {
        match self {
            Self::ExecuteCircuitOneZero(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitOneOne(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitOneTwo(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitTwoZero(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitTwoOne(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitTwoTwo(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitThreeZero(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitThreeOne(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitThreeTwo(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitFourZero(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitFourOne(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
            Self::ExecuteCircuitFourTwo(c) => {
                c.gen_proof(pp, pk, label).map_err(|e| e.into())
            }
        }
    }
}
