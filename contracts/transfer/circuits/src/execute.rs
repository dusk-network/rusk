// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use crossover::CircuitCrossover;
use input::{CircuitInput, WitnessInput};
use output::{CircuitOutput, WitnessOutput};

use anyhow::Result;
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::JubJubExtended;
use phoenix_core::Note;
use poseidon252::sponge;
use poseidon252::tree::{self, PoseidonBranch};
use rand_core::{CryptoRng, RngCore};
use schnorr::gadgets as schnorr_gadgets;

use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;

#[cfg(test)]
mod tests;

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Default, Clone)]
pub struct ExecuteCircuit<const DEPTH: usize, const CAPACITY: usize> {
    pi_positions: Vec<PublicInput>,
    pub inputs: Vec<CircuitInput<DEPTH>>,
    pub crossover: CircuitCrossover,
    pub outputs: Vec<CircuitOutput>,
    pub tx_hash: BlsScalar,
}

impl<const DEPTH: usize, const CAPACITY: usize>
    ExecuteCircuit<DEPTH, CAPACITY>
{
    pub const fn transcript_label(&self) -> &'static [u8] {
        b"execute-circuit"
    }

    pub fn rusk_keys_id(&self) -> String {
        format!(
            "transfer-execute-{}-{}",
            self.inputs.len(),
            self.outputs.len()
        )
    }

    pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
        self.tx_hash = tx_hash;
    }

    pub fn add_input<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        branch: PoseidonBranch<DEPTH>,
        sk_r: JubJubScalar,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
    ) -> Result<()> {
        let input = CircuitInput::new(
            rng,
            branch,
            sk_r,
            note,
            value,
            blinding_factor,
            nullifier,
        );

        self.inputs.push(input);

        Ok(())
    }

    pub fn set_crossover(
        &mut self,
        value_commitment: JubJubExtended,
        value: u64,
        blinding_factor: JubJubScalar,
    ) {
        self.crossover =
            CircuitCrossover::new(value_commitment, value, blinding_factor);
    }

    pub fn add_output(
        &mut self,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
    ) {
        let output = CircuitOutput::new(note, value, blinding_factor);
        self.outputs.push(output);
    }

    /// Constant message for the schnorr signature generation
    ///
    /// The signature is provided outside the circuit; so that's why it is
    /// constant
    ///
    /// The contents of the message are yet to be defined in the documentation.
    /// For now, it is treated as a constant.
    pub const fn sign_message() -> BlsScalar {
        BlsScalar::one()
    }
}

impl<const DEPTH: usize, const CAPACITY: usize> Circuit<'_>
    for ExecuteCircuit<DEPTH, CAPACITY>
{
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let mut pi = vec![];
        let mut base_root = None;

        // 1. Prove the knowledge of the input Note paths to Note Tree, via root
        // anchor
        let inputs: Vec<WitnessInput> = self
            .inputs
            .iter()
            .map(|input| {
                let branch = input.branch();
                let note = input.to_witness(composer);

                let note_hash = note.note_hash;
                let root_p = tree::merkle_opening(composer, branch, note_hash);

                // Test the public input only for the first root
                //
                // The remainder roots must be equal to the first (root is
                // unique per proof)
                match base_root {
                    None => {
                        let root = *branch.root();

                        pi.push(PublicInput::BlsScalar(
                            root,
                            composer.circuit_size(),
                        ));

                        composer.constrain_to_constant(
                            root_p,
                            BlsScalar::zero(),
                            -root,
                        );

                        base_root.replace(root_p);
                    }

                    Some(base) => {
                        composer.assert_equal(base, root_p);
                    }
                }

                note
            })
            .collect();

        // 2. Prove the knowledge of the pre-images of the input Notes
        inputs.iter().for_each(|input| {
            let note_hash = input.note_hash;
            let hash_inputs = input.to_hash_inputs();

            let note_hash_p = sponge::gadget(composer, &hash_inputs);

            composer.assert_equal(note_hash, note_hash_p);
        });

        // 3. Prove the correctness of the Schnorr signatures.
        inputs.iter().for_each(|input| {
            schnorr_gadgets::double_key_verify(
                composer,
                input.schnorr_r,
                input.schnorr_r_prime,
                input.schnorr_u,
                input.pk_r,
                input.pk_r_prime,
                input.schnorr_message,
            );
        });

        // 4. Prove the correctness of the nullifiers
        inputs.iter().for_each(|input| {
            let nullifier = input.nullifier;
            let sk_r = input.sk_r;
            let pos = input.pos;

            let nullifier_p = sponge::gadget(composer, &[sk_r, pos]);

            pi.push(PublicInput::BlsScalar(nullifier, composer.circuit_size()));
            composer.constrain_to_constant(
                nullifier_p,
                BlsScalar::zero(),
                -nullifier,
            );
        });

        // 5. Prove the knowledge of the commitment openings of the commitments
        // of the input Notes
        inputs.iter().for_each(|input| {
            let value_commitment = input.value_commitment;
            let value_commitment_p = gadgets::commitment(
                composer,
                input.value,
                input.blinding_factor,
            );

            composer.assert_equal_point(value_commitment, value_commitment_p);
        });

        // 6. Prove that the value of the openings of the commitments of the
        // input Notes is in range
        inputs.iter().for_each(|input| {
            composer.range_gate(input.value, 64);
        });

        // 7. Prove the knowledge of the commitment opening of the Crossover
        let crossover = self.crossover.to_witness(composer);
        {
            let value_commitment_p = gadgets::commitment(
                composer,
                crossover.value,
                crossover.blinding_factor,
            );

            // fee value public input
            pi.push(PublicInput::BlsScalar(
                crossover.fee_value,
                composer.circuit_size(),
            ));

            composer.constrain_to_constant(
                crossover.value,
                BlsScalar::zero(),
                -crossover.fee_value,
            );

            // value commitment public input
            let value_commitment = crossover.value_commitment.into();
            pi.push(PublicInput::AffinePoint(
                value_commitment,
                composer.circuit_size(),
                composer.circuit_size() + 1,
            ));

            composer.assert_equal_public_point(
                value_commitment_p,
                value_commitment,
            );
        }

        // 8. Prove that the value of the opening of the commitment of the
        // Crossover is within range
        composer.range_gate(crossover.value, 64);

        // 9. Prove the knowledge of the commitment openings of the commitments
        // of the output Obfuscated Notes
        let outputs: Vec<WitnessOutput> = self
            .outputs
            .iter()
            .map(|output| {
                let output = output.to_witness(composer);

                let value_commitment_p = gadgets::commitment(
                    composer,
                    output.value,
                    output.blinding_factor,
                );

                // value commitment public input
                let value_commitment = output.value_commitment.into();
                pi.push(PublicInput::AffinePoint(
                    value_commitment,
                    composer.circuit_size(),
                    composer.circuit_size() + 1,
                ));

                composer.assert_equal_public_point(
                    value_commitment_p,
                    value_commitment,
                );

                output
            })
            .collect();

        // 10. Prove that the value of the openings of the commitments of the
        // output Obfuscated Notes is in range
        outputs.iter().for_each(|output| {
            composer.range_gate(output.value, 64);
        });

        // 11. Prove that sum(inputs.value) - sum(outputs.value) -
        // crossover_value = 0
        {
            let zero =
                composer.add_witness_to_circuit_description(BlsScalar::zero());

            let inputs_sum = inputs.iter().fold(zero, |sum, input| {
                composer.add(
                    (BlsScalar::one(), sum),
                    (BlsScalar::one(), input.value),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                )
            });

            let outputs_sum = outputs.iter().fold(zero, |sum, output| {
                composer.add(
                    (BlsScalar::one(), sum),
                    (BlsScalar::one(), output.value),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                )
            });

            composer.poly_gate(
                inputs_sum,
                outputs_sum,
                crossover.value,
                BlsScalar::zero(),
                BlsScalar::one(),
                -BlsScalar::one(),
                -BlsScalar::one(),
                BlsScalar::zero(),
                BlsScalar::zero(),
            );
        }

        // 12. Inject the transaction hash to tie it to the circuit
        //
        // This is a workaround while the transcript hash injection is not
        // available in the API.
        //
        // This step is necessary to guarantee the outputs were not tampered by
        // a malicious actor. It is cheaper than checking individually
        // for the pre-image of every output.
        let tx_hash = composer.add_input(self.tx_hash);
        pi.push(PublicInput::BlsScalar(
            self.tx_hash,
            composer.circuit_size(),
        ));

        composer.constrain_to_constant(
            tx_hash,
            BlsScalar::zero(),
            -self.tx_hash,
        );

        self.get_mut_pi_positions().extend_from_slice(pi.as_slice());

        Ok(())
    }

    fn get_trim_size(&self) -> usize {
        1 << CAPACITY
    }

    fn set_trim_size(&mut self, _size: usize) {
        // N/A, fixed size circuit
    }

    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    /// Return a reference to the Public Inputs storage of the circuit.
    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }
}
