// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{CircuitCrossover, CircuitInput, CircuitOutput, ExecuteCircuit};
use crate::error::Error;
use crate::gadgets;

use dusk_jubjub::{GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED};
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::sponge;
use phoenix_core::{Crossover, Fee, Note};

use dusk_plonk::prelude::*;

macro_rules! execute_circuit_variant {
    ($i:ident,$c:expr) => {
        /// The circuit responsible for creating a zero-knowledge proof
        #[derive(Debug, Default, Clone)]
        pub struct $i {
            inputs: Vec<CircuitInput>,
            crossover: CircuitCrossover,
            outputs: Vec<CircuitOutput>,
            tx_hash: BlsScalar,
        }

        impl $i {
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

            pub fn add_input(&mut self, input: CircuitInput) {
                self.inputs.push(input);
            }

            pub const fn tx_hash(&self) -> &BlsScalar {
                &self.tx_hash
            }

            pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
                self.tx_hash = tx_hash;
            }

            pub fn set_fee(&mut self, fee: &Fee) -> Result<(), Error> {
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

                Ok(())
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

            pub fn add_output_with_data(
                &mut self,
                note: Note,
                value: u64,
                blinding_factor: JubJubScalar,
            ) {
                let output = CircuitOutput::new(note, value, blinding_factor);

                self.outputs.push(output);
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
        }

        impl TryFrom<ExecuteCircuit> for $i {
            type Error = Error;

            fn try_from(c: ExecuteCircuit) -> Result<Self, Self::Error> {
                match c {
                    ExecuteCircuit::$i(c) => Ok(c),
                    _ => Err(Error::IncorrectExecuteCircuitVariant(
                        c.inputs().len(),
                        c.outputs().len(),
                    )),
                }
            }
        }

        #[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
        impl Circuit for $i {
            fn gadget(
                &mut self,
                composer: &mut TurboComposer,
            ) -> Result<(), PlonkError> {
                let _ = $i::CIRCUIT_ID;

                let zero = TurboComposer::constant_zero();

                // Set the common root/anchor for all inputs
                let tx_hash = *self.tx_hash();
                let tx_hash = composer.append_public_witness(tx_hash);

                let anchor = self.anchor();
                let anchor = composer.append_public_witness(anchor);

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
                            );

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
                            );

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
                );

                composer.assert_equal_constant(
                    crossover.fee_value_witness,
                    BlsScalar::zero(),
                    Some(-crossover.fee_value),
                );

                // 3. ∀(o,v) ∈ O × V | O → V
                let outputs = self.outputs.iter().fold(
                    TurboComposer::constant_zero(),
                    |sum, output| {
                        let witness = output.to_witness(composer);
                        let commitment = composer
                            .append_public_point(witness.value_commitment);

                        // 1.a commitment(oc,ov,ob,64)
                        gadgets::commitment(
                            composer,
                            commitment,
                            witness.value,
                            witness.blinding_factor,
                            64,
                        );

                        let constraint = Constraint::new()
                            .left(1)
                            .a(sum)
                            .right(1)
                            .b(witness.value);

                        composer.gate_add(constraint)
                    },
                );

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

            fn public_inputs(&self) -> Vec<PublicInputValue> {
                // 1.a opening(io,A,ih)
                let mut pi = vec![self.tx_hash.into(), self.anchor().into()];

                // 1.f n == H(ik',ip)
                let nullifiers = self
                    .inputs
                    .iter()
                    .map(CircuitInput::nullifier)
                    .cloned()
                    .map(|i| i.into());

                pi.extend(nullifiers);

                // 2. commitment(Cc,cv,cb,64)
                let crossover =
                    JubJubAffine::from(self.crossover.value_commitment());
                pi.push(crossover.into());

                pi.push(BlsScalar::from(self.crossover.fee()).into());

                // 3. ∀(o,v) ∈ O × V | O → V
                let outputs = self.outputs.iter().map(|output| {
                    JubJubAffine::from(output.note().value_commitment()).into()
                });

                pi.extend(outputs);

                pi
            }

            fn padded_gates(&self) -> usize {
                1 << $c
            }
        }
    };
}

execute_circuit_variant!(ExecuteCircuitOneZero, 15);
execute_circuit_variant!(ExecuteCircuitOneOne, 15);
execute_circuit_variant!(ExecuteCircuitOneTwo, 16);
execute_circuit_variant!(ExecuteCircuitTwoZero, 16);
execute_circuit_variant!(ExecuteCircuitTwoOne, 16);
execute_circuit_variant!(ExecuteCircuitTwoTwo, 16);
execute_circuit_variant!(ExecuteCircuitThreeZero, 17);
execute_circuit_variant!(ExecuteCircuitThreeOne, 17);
execute_circuit_variant!(ExecuteCircuitThreeTwo, 17);
execute_circuit_variant!(ExecuteCircuitFourZero, 17);
execute_circuit_variant!(ExecuteCircuitFourOne, 17);
execute_circuit_variant!(ExecuteCircuitFourTwo, 17);
