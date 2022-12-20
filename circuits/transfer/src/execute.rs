// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{error::Error, gadgets, POSEIDON_TREE_DEPTH};

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_poseidon::sponge;
use dusk_poseidon::tree::{PoseidonBranch, PoseidonLeaf, PoseidonTree};
use nstack::annotation::Keyed;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;

pub use crossover::{CircuitCrossover, WitnessCrossover};
pub use input::{CircuitInput, CircuitInputSignature, WitnessInput};
pub use output::{CircuitOutput, WitnessOutput};

#[cfg(feature = "builder")]
pub mod builder;

pub enum ExecuteCircuit {
    OneTwo(ExecuteCircuitOneTwo),
    TwoTwo(ExecuteCircuitTwoTwo),
    ThreeTwo(ExecuteCircuitThreeTwo),
    FourTwo(ExecuteCircuitFourTwo),
}

impl ExecuteCircuit {
    /// Create a new circuit with the given number of inputs.
    ///
    /// # Panics
    /// If the number of inputs is not in the 1..=4 range.
    pub fn new(inputs: usize) -> Self {
        match inputs {
            1 => Self::OneTwo(ExecuteCircuitOneTwo::default()),
            2 => Self::TwoTwo(ExecuteCircuitTwoTwo::default()),
            3 => Self::ThreeTwo(ExecuteCircuitThreeTwo::default()),
            4 => Self::FourTwo(ExecuteCircuitFourTwo::default()),
            _ => panic!("Number of inputs not supported"),
        }
    }

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

    pub fn input_branch<L, K>(
        tree: &PoseidonTree<L, K, { POSEIDON_TREE_DEPTH }>,
        pos: u64,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Error>
    where
        L: PoseidonLeaf + Keyed<K> + Into<Note>,
        K: Clone + PartialOrd,
    {
        tree.branch(pos).ok_or(Error::NoSuchBranch)
    }

    pub fn input<R, L, K>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        tx_hash: BlsScalar,
        tree: &PoseidonTree<L, K, { POSEIDON_TREE_DEPTH }>,
        note: Note,
    ) -> Result<CircuitInput, Error>
    where
        R: RngCore + CryptoRng,
        L: PoseidonLeaf + Keyed<K> + Into<Note>,
        K: Clone + PartialOrd,
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

    pub fn prove<Rng: RngCore + CryptoRng>(
        &mut self,
        rng: &mut Rng,
        prover_key: &[u8],
    ) -> Result<(Proof, Vec<BlsScalar>), Error> {
        self.pad();

        Ok(match self {
            ExecuteCircuit::OneTwo(c) => {
                let prover = Prover::try_from_bytes(prover_key)?;
                prover.prove(rng, c)?
            }
            ExecuteCircuit::TwoTwo(c) => {
                let prover = Prover::try_from_bytes(prover_key)?;
                prover.prove(rng, c)?
            }
            ExecuteCircuit::ThreeTwo(c) => {
                let prover = Prover::try_from_bytes(prover_key)?;
                prover.prove(rng, c)?
            }
            ExecuteCircuit::FourTwo(c) => {
                let prover = Prover::try_from_bytes(prover_key)?;
                prover.prove(rng, c)?
            }
        })
    }

    pub fn circuit_id(&self) -> &'static [u8; 32] {
        match self {
            ExecuteCircuit::OneTwo(_) => ExecuteCircuitOneTwo::circuit_id(),
            ExecuteCircuit::TwoTwo(_) => ExecuteCircuitTwoTwo::circuit_id(),
            ExecuteCircuit::ThreeTwo(_) => ExecuteCircuitThreeTwo::circuit_id(),
            ExecuteCircuit::FourTwo(_) => ExecuteCircuitFourTwo::circuit_id(),
        }
    }

    pub fn pad(&mut self) {
        match self {
            ExecuteCircuit::OneTwo(c) => c.pad(),
            ExecuteCircuit::TwoTwo(c) => c.pad(),
            ExecuteCircuit::ThreeTwo(c) => c.pad(),
            ExecuteCircuit::FourTwo(c) => c.pad(),
        }
    }

    pub fn add_output_with_data(
        &mut self,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
    ) {
        match self {
            ExecuteCircuit::OneTwo(c) => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            ExecuteCircuit::TwoTwo(c) => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            ExecuteCircuit::ThreeTwo(c) => {
                c.add_output_with_data(note, value, blinding_factor)
            }
            ExecuteCircuit::FourTwo(c) => {
                c.add_output_with_data(note, value, blinding_factor)
            }
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
            ExecuteCircuit::OneTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            ExecuteCircuit::TwoTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            ExecuteCircuit::ThreeTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
            ExecuteCircuit::FourTwo(c) => {
                c.set_fee_crossover(fee, crossover, value, blinder)
            }
        }
    }

    pub fn set_fee(&mut self, fee: &Fee) {
        match self {
            ExecuteCircuit::OneTwo(c) => c.set_fee(fee),
            ExecuteCircuit::TwoTwo(c) => c.set_fee(fee),
            ExecuteCircuit::ThreeTwo(c) => c.set_fee(fee),
            ExecuteCircuit::FourTwo(c) => c.set_fee(fee),
        }
    }

    pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
        match self {
            ExecuteCircuit::OneTwo(c) => c.set_tx_hash(tx_hash),
            ExecuteCircuit::TwoTwo(c) => c.set_tx_hash(tx_hash),
            ExecuteCircuit::ThreeTwo(c) => c.set_tx_hash(tx_hash),
            ExecuteCircuit::FourTwo(c) => c.set_tx_hash(tx_hash),
        }
    }

    pub fn add_input(&mut self, input: CircuitInput) {
        match self {
            ExecuteCircuit::OneTwo(c) => c.add_input(input),
            ExecuteCircuit::TwoTwo(c) => c.add_input(input),
            ExecuteCircuit::ThreeTwo(c) => c.add_input(input),
            ExecuteCircuit::FourTwo(c) => c.add_input(input),
        }
    }
}

macro_rules! execute_circuit_variant {
    ($ty:ident) => {
        /// The circuit responsible for creating a zero-knowledge proof
        #[derive(Debug, Default, Clone)]
        pub struct $ty {
            inputs: Vec<CircuitInput>,
            crossover: CircuitCrossover,
            outputs: Vec<CircuitOutput>,
            tx_hash: BlsScalar,
        }

        impl $ty {
            pub fn add_output_with_data(
                &mut self,
                note: Note,
                value: u64,
                blinding_factor: JubJubScalar,
            ) {
                let output = CircuitOutput::new(note, value, blinding_factor);

                self.outputs.push(output);
            }

            pub fn set_fee_crossover(
                &mut self,
                fee: &Fee,
                crossover: &Crossover,
                value: u64,
                blinder: JubJubScalar,
            ) {
                let value_commitment = *crossover.value_commitment();
                let fee = fee.gas_limit * fee.gas_price;

                self.crossover = CircuitCrossover::new(
                    value_commitment,
                    value,
                    blinder,
                    fee,
                );
            }

            pub fn set_fee(&mut self, fee: &Fee) {
                let value = 0;
                let blinding_factor = JubJubScalar::zero();
                let value_commitment = (GENERATOR_EXTENDED
                    * JubJubScalar::zero())
                    + (GENERATOR_NUMS_EXTENDED * blinding_factor);

                let fee = fee.gas_limit * fee.gas_price;
                self.crossover = CircuitCrossover::new(
                    value_commitment,
                    value,
                    blinding_factor,
                    fee,
                );
            }

            pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
                self.tx_hash = tx_hash;
            }

            pub fn add_input(&mut self, input: CircuitInput) {
                self.inputs.push(input);
            }
        }

        impl $ty {
            pub const fn new(
                inputs: Vec<CircuitInput>,
                crossover: CircuitCrossover,
                outputs: Vec<CircuitOutput>,
                tx_hash: BlsScalar,
            ) -> Self {
                Self {
                    inputs,
                    crossover,
                    outputs,
                    tx_hash,
                }
            }

            pub fn into_inner(
                &self,
            ) -> (
                Vec<CircuitInput>,
                CircuitCrossover,
                Vec<CircuitOutput>,
                BlsScalar,
            ) {
                let inputs = self.inputs.clone();
                let crossover = self.crossover.clone();
                let outputs = self.outputs.clone();
                let tx_hash = self.tx_hash.clone();

                (inputs, crossover, outputs, tx_hash)
            }

            pub const fn tx_hash(&self) -> &BlsScalar {
                &self.tx_hash
            }

            /// Return the anchor root of the inputs.
            ///
            /// The circuit expects a single root for all the inputs.
            ///
            /// It will return `BlsScalar::default` in case no inputs are
            /// provided to the circuit.
            pub fn anchor(&self) -> BlsScalar {
                self.inputs
                    .first()
                    .map(|i| i.branch().root())
                    .copied()
                    .unwrap_or_default()
            }

            pub fn inputs(&self) -> &[CircuitInput] {
                self.inputs.as_slice()
            }

            pub fn outputs(&self) -> &[CircuitOutput] {
                self.outputs.as_slice()
            }

            pub fn pad(&mut self) {
                while self.outputs.len() < 2 {
                    self.outputs.push(CircuitOutput::pad());
                }
            }

            pub fn public_inputs(&self) -> Vec<BlsScalar> {
                // 1.a opening(io,A,ih)
                let mut pi = vec![self.tx_hash.into(), self.anchor().into()];

                // 1.f n == H(ik',ip)
                let nullifiers =
                    self.inputs.iter().map(CircuitInput::nullifier).cloned();

                pi.extend(nullifiers);

                // 2. commitment(Cc,cv,cb,64)
                let crossover =
                    JubJubAffine::from(self.crossover.value_commitment());
                pi.extend([crossover.get_x(), crossover.get_y()]);

                pi.push(BlsScalar::from(self.crossover.fee()).into());

                // 3. ∀(o,v) ∈ O × V | O → V
                let mut outputs = Vec::with_capacity(2 * self.outputs.len());
                for output in self.outputs.iter() {
                    let commitment =
                        JubJubAffine::from(output.note().value_commitment());
                    outputs.extend([commitment.get_x(), commitment.get_y()]);
                }

                pi.extend(outputs);

                pi
            }

            pub fn add_output(
                &mut self,
                note: Note,
                vk: Option<&ViewKey>,
            ) -> Result<(), Error> {
                let value = note.value(vk)?;
                let blinding_factor = note.blinding_factor(vk)?;

                Ok(self.add_output_with_data(note, value, blinding_factor))
            }

            pub const fn circuit_id() -> &'static [u8; 32] {
                &Self::CIRCUIT_ID
            }
        }

        #[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
        impl Circuit for $ty {
            fn circuit<C: Composer>(
                &self,
                composer: &mut C,
            ) -> Result<(), PlonkError> {
                let zero = C::ZERO;

                // Set the common root/anchor for all inputs
                let tx_hash = *self.tx_hash();
                let tx_hash = composer.append_public(tx_hash);

                let anchor = self.anchor();
                let anchor = composer.append_public(anchor);

                // 1. ∀(i, n) ∈ I × N | I → N
                let inputs = self
                    .inputs
                    .iter()
                    .try_fold::<_, _, Result<Witness, Error>>(
                        zero,
                        |sum, input| {
                            let witness = input.to_witness(composer)?;

                            // 1.a opening(io,A,ih)
                            gadgets::merkle_opening(
                                composer,
                                input.branch(),
                                anchor,
                                witness.note_hash,
                            );

                            // 1.b ih == H(it,ic,in,ik,ir,ip,iψ)
                            let hash = witness.to_hash_inputs();
                            let hash = sponge::gadget(composer, &hash);
                            composer.assert_equal(witness.note_hash, hash);

                            // 1.c doubleSchnorrVerify(iσ,ik,T)
                            gadgets::schnorr_double_key_verify(
                                composer,
                                witness.schnorr_u,
                                witness.schnorr_r,
                                witness.schnorr_r_p,
                                witness.pk_r,
                                witness.pk_r_p,
                                tx_hash,
                            )?;

                            // 1.d n == H(ik',ip)
                            let n = [
                                *witness.pk_r_p.x(),
                                *witness.pk_r_p.y(),
                                witness.pos,
                            ];
                            let n = sponge::gadget(composer, &n);
                            composer.assert_equal_constant(
                                n,
                                BlsScalar::zero(),
                                Some(-witness.nullifier),
                            );

                            // 1.e commitment(ic,iv,ib,64)
                            gadgets::commitment(
                                composer,
                                witness.value_commitment,
                                witness.value,
                                witness.blinding_factor,
                                64,
                            )?;

                            let constraint = Constraint::new()
                                .left(1)
                                .a(sum)
                                .right(1)
                                .b(witness.value);

                            Ok(composer.gate_add(constraint))
                        },
                    )
                    .or(Err(PlonkError::CircuitInputsNotFound))?;

                // 2. commitment(Cc,cv,cb,64)
                let crossover = self.crossover.to_witness(composer);
                let commitment =
                    composer.append_public_point(crossover.value_commitment);

                gadgets::commitment(
                    composer,
                    commitment,
                    crossover.value,
                    crossover.blinding_factor,
                    64,
                )?;

                composer.assert_equal_constant(
                    crossover.fee_value_witness,
                    BlsScalar::zero(),
                    Some(-crossover.fee_value),
                );

                // 3. ∀(o,v) ∈ O × V | O → V
                let mut outputs = C::ZERO;
                for output in self.outputs.iter() {
                    let witness = output.to_witness(composer);
                    let commitment =
                        composer.append_public_point(witness.value_commitment);

                    // 1.a commitment(oc,ov,ob,64)
                    gadgets::commitment(
                        composer,
                        commitment,
                        witness.value,
                        witness.blinding_factor,
                        64,
                    )?;

                    let constraint = Constraint::new()
                        .left(1)
                        .a(outputs)
                        .right(1)
                        .b(witness.value);

                    outputs = composer.gate_add(constraint);
                }

                // 4. ∑(iv ∈ I) − ∑(ov ∈ O) − cv − F = 0
                let constraint = Constraint::new()
                    .left(1)
                    .a(outputs)
                    .right(1)
                    .b(crossover.value)
                    .fourth(1)
                    .d(crossover.fee_value_witness);
                let o = composer.gate_add(constraint);

                composer.assert_equal(inputs, o);

                Ok(())
            }
        }
    };
}

execute_circuit_variant!(ExecuteCircuitOneTwo);
execute_circuit_variant!(ExecuteCircuitTwoTwo);
execute_circuit_variant!(ExecuteCircuitThreeTwo);
execute_circuit_variant!(ExecuteCircuitFourTwo);
