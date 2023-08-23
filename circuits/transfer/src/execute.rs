// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{error::Error, gadgets};

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_merkle::Aggregate;
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_poseidon::sponge;
use phoenix_core::{Crossover, Fee, Note};
use poseidon_merkle::{Opening, Tree};
use rand_core::{CryptoRng, RngCore};

use dusk_plonk::error::Error as PlonkError;
use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;

pub use crossover::{CircuitCrossover, WitnessCrossover};
pub use input::{CircuitInput, CircuitInputSignature, WitnessInput};
pub use output::{CircuitOutput, WitnessOutput};

pub type ExecuteCircuitOneTwo = ExecuteCircuit<1, (), 17, 4>;
pub type ExecuteCircuitTwoTwo = ExecuteCircuit<2, (), 17, 4>;
pub type ExecuteCircuitThreeTwo = ExecuteCircuit<3, (), 17, 4>;
pub type ExecuteCircuitFourTwo = ExecuteCircuit<4, (), 17, 4>;

const OUTPUTS: usize = 2;

/// The circuit responsible for creating a zero-knowledge proof
#[derive(Debug, Clone)]
pub struct ExecuteCircuit<const I: usize, T, const H: usize, const A: usize> {
    inputs: [Option<CircuitInput<T, H, A>>; I],
    crossover: CircuitCrossover,
    outputs: [Option<CircuitOutput>; OUTPUTS],
    tx_hash: BlsScalar,
}

impl<const I: usize, T, const H: usize, const A: usize>
    ExecuteCircuit<I, T, H, A>
{
    const NONE_INPUT: Option<CircuitInput<T, H, A>> = None;
    const NONE_OUTPUT: Option<CircuitOutput> = None;

    pub fn new() -> Self {
        Self {
            inputs: [Self::NONE_INPUT; I],
            crossover: CircuitCrossover::default(),
            outputs: [Self::NONE_OUTPUT; OUTPUTS],
            tx_hash: BlsScalar::zero(),
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

    pub fn input_branch(
        tree: &Tree<T, H, A>,
        pos: u64,
    ) -> Result<Opening<T, H, A>, Error>
    where
        T: Clone + Aggregate<A>,
    {
        tree.opening(pos).ok_or(Error::NoSuchBranch)
    }

    pub fn input<R>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        tx_hash: BlsScalar,
        tree: &Tree<T, H, A>,
        note: Note,
    ) -> Result<CircuitInput<T, H, A>, Error>
    where
        T: Clone + Aggregate<A>,
        R: RngCore + CryptoRng,
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

    pub fn add_output_with_data(
        &mut self,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
    ) -> Result<(), Error> {
        for o in self.outputs.iter_mut() {
            if o.is_none() {
                let output = CircuitOutput::new(note, value, blinding_factor);
                *o = Some(output);
                return Ok(());
            }
        }

        Err(Error::CircuitMaximumOutputs)
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

        self.crossover =
            CircuitCrossover::new(value_commitment, value, blinder, fee);
    }

    pub fn set_fee(&mut self, fee: &Fee) {
        let value = 0;
        let blinding_factor = JubJubScalar::zero();
        let value_commitment = (GENERATOR_EXTENDED * JubJubScalar::zero())
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

    pub fn add_input(
        &mut self,
        input: CircuitInput<T, H, A>,
    ) -> Result<(), Error> {
        for i in self.inputs.iter_mut() {
            if i.is_none() {
                *i = Some(input);
                return Ok(());
            }
        }

        Err(Error::CircuitMaximumInputs)
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
    pub fn anchor(&self) -> BlsScalar
    where
        T: Clone + Aggregate<A>,
    {
        self.inputs()
            .first()
            .map(|i| i.branch().root().hash)
            .unwrap_or_default()
    }

    pub fn inputs(&self) -> Vec<&CircuitInput<T, H, A>> {
        let mut inputs = Vec::new();
        self.inputs.iter().for_each(|input| match input {
            Some(i) => inputs.push(i),
            None => {}
        });
        inputs
    }

    pub fn outputs(&self) -> Vec<&CircuitOutput> {
        let mut outputs = Vec::new();
        self.outputs.iter().for_each(|output| match output {
            Some(o) => outputs.push(o),
            None => {}
        });
        outputs
    }

    pub fn public_inputs(&self) -> Vec<BlsScalar>
    where
        T: Clone + Aggregate<A>,
    {
        // 1.a opening(io,A,ih)
        let mut pi = vec![self.tx_hash, self.anchor()];

        // 1.f n == H(ik',ip)
        let nullifiers = self
            .inputs()
            .into_iter()
            .map(CircuitInput::nullifier)
            .cloned();

        pi.extend(nullifiers);

        // 2. commitment(Cc,cv,cb,64)
        let crossover = JubJubAffine::from(self.crossover.value_commitment());
        pi.extend([crossover.get_x(), crossover.get_y()]);

        pi.push(BlsScalar::from(self.crossover.fee()));

        // 3. ∀(o,v) ∈ O × V | O → V
        let mut outputs = Vec::with_capacity(2 * self.outputs.len());
        for output in self.outputs().iter() {
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

        self.add_output_with_data(note, value, blinding_factor)
    }
}

impl<const I: usize, T, const H: usize, const A: usize> Default
    for ExecuteCircuit<I, T, H, A>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const I: usize, T, const H: usize, const A: usize>
    ExecuteCircuit<I, T, H, A>
where
    T: Default + Clone + Aggregate<A>,
{
    pub fn circuit<C: Composer>(
        &self,
        composer: &mut C,
    ) -> Result<(), PlonkError> {
        if self.inputs.len() != I {
            // TODO: change into InvalidCircuitSize error once plonk v0.15 is
            // merged across the stack
            return Err(PlonkError::CircuitInputsNotFound);
        }

        // Set the common root/anchor for all inputs
        let tx_hash = *self.tx_hash();
        let tx_hash = composer.append_public(tx_hash);

        let anchor = self.anchor();
        let anchor = composer.append_public(anchor);

        // 1. ∀(i, n) ∈ I × N | I → N
        let inputs = self
            .inputs()
            .iter()
            .try_fold::<_, _, Result<Witness, Error>>(C::ZERO, |sum, input| {
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
                let n = [*witness.pk_r_p.x(), *witness.pk_r_p.y(), witness.pos];
                let n = sponge::gadget(composer, &n);
                composer.assert_equal_constant(
                    n,
                    BlsScalar::zero(),
                    Some(witness.nullifier),
                );

                // 1.e commitment(ic,iv,ib,64)
                gadgets::commitment(
                    composer,
                    witness.value_commitment,
                    witness.value,
                    witness.blinding_factor,
                    64,
                )?;

                let constraint =
                    Constraint::new().left(1).a(sum).right(1).b(witness.value);

                Ok(composer.gate_add(constraint))
            })
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
            Some(crossover.fee_value),
        );

        // 3. ∀(o,v) ∈ O × V | O → V
        let mut outputs = C::ZERO;
        for o in self.outputs.as_ref() {
            let padded_output = CircuitOutput::pad();
            let output: &CircuitOutput = o.as_ref().unwrap_or(&padded_output);
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

#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for ExecuteCircuitOneTwo {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        self.circuit(composer)
    }
}
#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for ExecuteCircuitTwoTwo {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        self.circuit(composer)
    }
}
#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for ExecuteCircuitThreeTwo {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        self.circuit(composer)
    }
}
#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for ExecuteCircuitFourTwo {
    fn circuit<C: Composer>(&self, composer: &mut C) -> Result<(), PlonkError> {
        self.circuit(composer)
    }
}

impl ExecuteCircuitOneTwo {
    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}
impl ExecuteCircuitTwoTwo {
    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}
impl ExecuteCircuitThreeTwo {
    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}
impl ExecuteCircuitFourTwo {
    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}
