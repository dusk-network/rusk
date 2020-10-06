// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets::secret_key::sk_knowledge;
use crate::gadgets::{
    merkle::merkle, nullifier::nullifier_gadget, range::range,
};
use anyhow::Result;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use kelvin::Blake2b;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use poseidon252::{PoseidonAnnotation, PoseidonBranch, PoseidonTree};

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Default, Clone)]
pub struct ExecuteCircuit {
    /// Storage height of the tree
    // pub anchor: Option<BlsScalar>,
    /// Nullifier for note
    pub nullifiers: Vec<BlsScalar>,
    /// Note hashes
    pub note_hashes: Vec<BlsScalar>,
    /// Positions of notes
    pub position_of_notes: Vec<BlsScalar>,
    /// Poseidon branches of the input notes
    pub input_poseidon_branches: Vec<PoseidonBranch>,
    /// Input notes secret keys
    pub input_notes_sk: Vec<JubJubScalar>,
    /// Input notes public keys
    pub input_notes_pk: Vec<AffinePoint>,
    /// Input commitment points
    pub input_commitments: Vec<AffinePoint>,
    /// Input note values
    pub input_values: Vec<BlsScalar>,
    /// Input notes blinders
    pub input_blinders: Vec<BlsScalar>,
    /// Commitment point to crossover
    pub crossover_commitment: AffinePoint,
    /// Crossover commitment value
    pub crossover_commitment_value: BlsScalar,
    /// Crossover commitment blinder
    pub crossover_commitment_blinder: BlsScalar,
    /// Obfuscated note commitments
    pub obfuscated_commitment_points: Vec<AffinePoint>,
    /// Obfuscated note values
    pub obfuscated_note_values: Vec<BlsScalar>,
    /// Obfuscated note blinder
    pub obfuscated_note_blinders: Vec<BlsScalar>,
    /// Fee
    pub fee: BlsScalar,
    /// Returns circuit size
    pub trim_size: usize,
    /// Gives Public Inputs
    pub pi_positions: Vec<PublicInput>,
}

impl Circuit<'_> for ExecuteCircuit {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        // XXX: The anchors do not seem necessary, as they are contained within the poseidon branch
        // XXX: but until they are removed from the specs, they will remain commented here.
        // let anchor = self
        //     .anchor
        //     .as_ref()
        //
        let nullifiers = self.nullifiers.clone();
        let note_hashes: Vec<AllocatedScalar> = self
            .note_hashes
            .iter()
            .map(|note_hash| AllocatedScalar::allocate(composer, *note_hash))
            .collect();
        let position_of_notes: Vec<AllocatedScalar> = self
            .position_of_notes
            .iter()
            .map(|position_of_notes| {
                AllocatedScalar::allocate(composer, *position_of_notes)
            })
            .collect();
        let input_poseidon_branches = self.input_poseidon_branches.clone();
        let input_notes_sk: Vec<AllocatedScalar> = self
            .input_notes_sk
            .iter()
            .map(|input_notes_sk| {
                AllocatedScalar::allocate(
                    composer,
                    BlsScalar::from(*input_notes_sk),
                )
            })
            .collect();
        let input_notes_pk: Vec<PlonkPoint> = self
            .input_notes_pk
            .iter()
            .map(|input_notes_pk| {
                PlonkPoint::from_private_affine(composer, *input_notes_pk)
            })
            .collect();
        let input_commitments: Vec<PlonkPoint> = self
            .input_commitments
            .iter()
            .map(|input_commitments| {
                PlonkPoint::from_private_affine(composer, *input_commitments)
            })
            .collect();
        let mut input_note_values: Vec<AllocatedScalar> = self
            .input_values
            .iter()
            .map(|input_values| {
                AllocatedScalar::allocate(composer, *input_values)
            })
            .collect();
        let input_notes_blinders: Vec<AllocatedScalar> = self
            .input_blinders
            .iter()
            .map(|input_blinders| {
                AllocatedScalar::allocate(composer, *input_blinders)
            })
            .collect();
        let crossover_commitment = self.crossover_commitment;
        let crossover_commitment_value = self.crossover_commitment_value;
        let crossover_commitment_blinder = self.crossover_commitment_blinder;
        let obfuscated_commitment_points =
            self.obfuscated_commitment_points.clone();
        let mut obfuscated_note_values: Vec<AllocatedScalar> = self
            .obfuscated_note_values
            .iter()
            .map(|obfuscated_note_values| {
                AllocatedScalar::allocate(composer, *obfuscated_note_values)
            })
            .collect();
        let obfuscated_note_blinders: Vec<AllocatedScalar> = self
            .obfuscated_note_blinders
            .iter()
            .map(|obfuscated_note_blinders| {
                AllocatedScalar::allocate(composer, *obfuscated_note_blinders)
            })
            .collect();
        let fee = self.fee;
        let pi = self.get_mut_pi_positions();

        let crossover_value =
            AllocatedScalar::allocate(composer, crossover_commitment_value);
        let crossover_blinder =
            AllocatedScalar::allocate(composer, crossover_commitment_blinder);

        // 1. Prove the knowledge of the input Note paths to Note Tree, via root anchor

        // Iterate over the branch of each note and push the roots into the
        // vector of public inputs
        input_poseidon_branches
            .iter()
            .zip(note_hashes.iter())
            .for_each(|(branch, note_hash)| {
                let root = merkle(composer, branch.clone(), *note_hash);

                pi.push(PublicInput::BlsScalar(
                    branch.root(),
                    composer.circuit_size(),
                ));

                composer.constrain_to_constant(
                    root,
                    BlsScalar::zero(),
                    -branch.root(),
                );
            });

        // 2. Prove the knowledge of the pre-images of the input Notes

        // Iterate over the note elements and hash them together
        // and constrain against the hash of the note.
        input_commitments
            .iter()
            .zip(position_of_notes.iter())
            .zip(input_notes_pk.iter())
            .zip(note_hashes.iter())
            .for_each(|(((value, position), key), note_hash)| {
                let computed_hash = sponge_hash_gadget(
                    composer,
                    &[*value.x(), *value.y(), position.var, *key.x(), *key.y()],
                );

                composer.assert_equal(computed_hash, note_hash.var);
            });

        // 3. Prove the knowledge of the secret keys corresponding to the public keys in input Notes
        // This is the notes sk_r and pk_r.

        // Iterate over each element of the vector of secret keys
        // and prove that they are relates to the the elements of
        // a vector pf public keys.
        input_notes_sk.iter().zip(input_notes_pk.iter()).for_each(
            |(secret_key, public_key)| {
                sk_knowledge(composer, *secret_key, *public_key);
            },
        );

        // 4. Prove the correctness of the nullifiers

        input_notes_sk
            .iter()
            .zip(position_of_notes.iter())
            .zip(nullifiers.iter())
            .for_each(|((sk, note_position), nullifier)| {
                let computed_nullifier =
                    nullifier_gadget(composer, *note_position, *sk);

                // Push Public nullifiers
                pi.push(PublicInput::BlsScalar(
                    *nullifier,
                    composer.circuit_size(),
                ));

                // Assert generated nullifiers are equal to publicly inputted nullifiers
                composer.constrain_to_constant(
                    computed_nullifier,
                    BlsScalar::zero(),
                    -nullifier,
                );
            });

        // 5. Prove the knowledge of the commitment openings of the commitments of the input Notes
        input_note_values
            .iter()
            .zip(input_notes_blinders.iter())
            .zip(input_commitments.iter())
            .for_each(|((value, blinder), input_commitment)| {
                let p1 = scalar_mul(composer, value.var, GENERATOR_EXTENDED);
                let p2 =
                    scalar_mul(composer, blinder.var, GENERATOR_NUMS_EXTENDED);

                let commitment = p1.point().fast_add(composer, *p2.point());

                // Assert computed commitment is equal to publicly inputted affine point
                composer.assert_equal_point(commitment, *input_commitment);
            });

        // 6. Prove that the value of the openings of the commitments of the input Notes is in range
        input_note_values.iter().for_each(|value| {
            range(composer, *value, 64);
        });

        // 7. Prove the knowledge of the commitment opening of the commitment of the Crossover
        let p3 = scalar_mul(composer, crossover_value.var, GENERATOR_EXTENDED);
        let p4 = scalar_mul(
            composer,
            crossover_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

        let commitment = p3.point().fast_add(composer, *p4.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            crossover_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, crossover_commitment);

        // 8. Prove that the value of the opening of the commitment of the Crossover is within range
        range(composer, crossover_value, 64);

        // 9. Prove the knowledge of the commitment openings of the commitments of the output Obfuscated Notes
        obfuscated_note_values
            .iter()
            .zip(obfuscated_note_blinders.iter())
            .zip(obfuscated_commitment_points.iter())
            .for_each(|((value, blinder), obfuscated_commitment_points)| {
                let p5 = scalar_mul(composer, value.var, GENERATOR_EXTENDED);
                let p6 =
                    scalar_mul(composer, blinder.var, GENERATOR_NUMS_EXTENDED);

                let commitment = p5.point().fast_add(composer, *p6.point());

                // Add PI constraint for the commitment computation check.
                pi.push(PublicInput::AffinePoint(
                    *obfuscated_commitment_points,
                    composer.circuit_size(),
                    composer.circuit_size() + 1,
                ));
                // Assert computed commitment is equal to publicly inputted affine point
                composer.assert_equal_public_point(
                    commitment,
                    *obfuscated_commitment_points,
                );
            });

        // 10. Prove that the value of the openings of the commitments of the output Obfuscated Notes is in range
        obfuscated_note_values.iter().for_each(|value| {
            range(composer, *value, 64);
        });

        // 11. Prove that input_note_value - output_note_value - crossover_value - fee = 0
        let zero =
            composer.add_witness_to_circuit_description(BlsScalar::zero());
        let initial = input_note_values[0].var;
        let all_input_values = input_note_values.iter_mut().skip(1).fold(
            initial,
            |acc, variable| {
                composer.add(
                    (BlsScalar::one(), acc),
                    (BlsScalar::one(), variable.var),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                )
            },
        );

        let initial = if obfuscated_note_values.is_empty() {
            zero
        } else {
            obfuscated_note_values[0].var
        };

        let all_obfuscated_values = obfuscated_note_values
            .iter_mut()
            .skip(1)
            .fold(initial, |acc, variable| {
                composer.add(
                    (BlsScalar::one(), acc),
                    (BlsScalar::one(), variable.var),
                    BlsScalar::zero(),
                    BlsScalar::zero(),
                )
            });

        pi.push(PublicInput::BlsScalar(fee, composer.circuit_size()));

        let crossover_commitment_value =
            AllocatedScalar::allocate(composer, crossover_commitment_value);
        composer.add_gate(
            crossover_commitment_value.var,
            all_obfuscated_values,
            all_input_values,
            BlsScalar::one(),
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            fee,
        );

        Ok(())
    }

    /// Returns the size at which we trim the `PublicParameters`
    /// to compile the circuit or perform proving/verification
    /// actions.
    fn get_trim_size(&self) -> usize {
        self.trim_size
    }

    fn set_trim_size(&mut self, size: usize) {
        self.trim_size = size;
    }

    /// /// Return a mutable reference to the Public Inputs storage of the circuit.
    fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
        &mut self.pi_positions
    }

    /// Return a reference to the Public Inputs storage of the circuit.
    fn get_pi_positions(&self) -> &Vec<PublicInput> {
        &self.pi_positions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use phoenix_core::{Note, NoteType};
    use poseidon252::PoseidonBranch;

    // Function to generate value commitment from value and blinder. This is a pedersen commitment.
    fn compute_value_commitment(
        value: JubJubScalar,
        blinder: JubJubScalar,
    ) -> AffinePoint {
        let commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * value)
                + &(GENERATOR_NUMS_EXTENDED * blinder),
        );

        commitment
    }

    // Function to build execute circuit from given circuit inputs
    fn build_execute_circuit(
        nullifiers: Vec<BlsScalar>,
        note_hashes: Vec<BlsScalar>,
        position_of_notes: Vec<BlsScalar>,
        input_poseidon_branches: Vec<PoseidonBranch>,
        input_notes_sk: Vec<JubJubScalar>,
        input_notes_pk: Vec<AffinePoint>,
        input_commitments: Vec<AffinePoint>,
        input_values: Vec<BlsScalar>,
        input_blinders: Vec<BlsScalar>,
        crossover_commitment: AffinePoint,
        crossover_commitment_value: BlsScalar,
        crossover_commitment_blinder: BlsScalar,
        obfuscated_commitment_points: Vec<AffinePoint>,
        obfuscated_note_values: Vec<BlsScalar>,
        obfuscated_note_blinders: Vec<BlsScalar>,
        fee: BlsScalar,
    ) -> ExecuteCircuit {
        ExecuteCircuit {
            // anchor: None,
            nullifiers: nullifiers,
            note_hashes: note_hashes,
            position_of_notes: position_of_notes,
            input_poseidon_branches: input_poseidon_branches,
            input_notes_sk: input_notes_sk,
            input_notes_pk: input_notes_pk,
            input_commitments: input_commitments,
            input_values: input_values,
            input_blinders: input_blinders,
            crossover_commitment: crossover_commitment,
            crossover_commitment_value: crossover_commitment_value.into(),
            crossover_commitment_blinder: crossover_commitment_blinder.into(),
            obfuscated_commitment_points: obfuscated_commitment_points,
            obfuscated_note_values: obfuscated_note_values,
            obfuscated_note_blinders: obfuscated_note_blinders,
            fee: fee,
            trim_size: 1 << 16,
            pi_positions: vec![],
        }
    }

    fn add_circuit_public_inputs(
        circuit: &ExecuteCircuit,
        crossover_commitment: AffinePoint,
        fee: BlsScalar,
        pi: &mut Vec<PublicInput>,
    ) {
        circuit.input_poseidon_branches.iter().for_each(|branch| {
            pi.push(PublicInput::BlsScalar(branch.root(), 0));
        });
        circuit.nullifiers.iter().for_each(|nullifier| {
            pi.push(PublicInput::BlsScalar(*nullifier, 0));
        });

        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));

        circuit.obfuscated_commitment_points.iter().for_each(
            |commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            },
        );

        pi.push(PublicInput::BlsScalar(-fee, 0));
    }

    fn circuit_note(
        ssk: SecretSpendKey,
        value: u64,
        pos: u64,
        input_note_blinder: JubJubScalar,
    ) -> Note {
        let r = JubJubScalar::from(150 as u64);
        let nonce = JubJubScalar::from(350 as u64);
        let psk = PublicSpendKey::from(&ssk);
        let mut note = Note::deterministic(
            NoteType::Transparent,
            &r,
            nonce,
            &psk,
            value,
            input_note_blinder,
        );
        note.set_pos(pos);
        note
    }

    #[test]
    // This test ensures the execute gadget is done correctly
    // by creating two notes and setting their field values
    // in the execute circuit
    fn test_execute_yes() -> Result<()> {
        // Generate the (a,b) for the note
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        // Declare the secret spend key for the note
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        // Assign the value of the note
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        // Create a deterministic note so that we can assign the blinder and not have inner randomness
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        // Set the position of the note
        note1.set_pos(0);
        // Derive the one time public key, pk_r, for the note
        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        // Generate the (a,b) for the note
        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        // Declare the secret spend key for the note
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        // Assign the value of the first note as 400, which is incorrect
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        // Create a deterministic note so that we can assign the blinder and not have inner randomness
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        // Set the position of the note
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        // Generate the value commitment of the note from the value and blinder
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        // Assign the postitions of the notes to a position in the tree
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        // Generate the crossover commitment, C.c(v,b)
        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        // Generate the commitment to the first output note, C.c(v,b)
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        // Generate the commitment to the second output note, C.c(v,b)
        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        // Assign the fee
        let fee = BlsScalar::from(200);

        let mut circuit = build_execute_circuit(
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk2)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![ obfuscated_note_value_one.into(), obfuscated_note_value_two.into()],

            vec![obfuscated_note_blinder_one.into(), obfuscated_note_blinder_two.into()],

            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 17, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        circuit.verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
    }

    #[test]
    // This circuit sets the wrong value for the first note,
    // which will fail the commitment check and create
    // an incorrect note. This is preventing the user from
    // falsifying their note value.
    fn test_wrong_note_value_one() -> Result<()> {
        // Generate the (a,b) for the note
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        // Declare the secret spend key for the note
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        // Assign the value of the first note as 400, which is incorrect
        let value1 = 500u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        // Create a deterministic note so that we can assign the blinder and not have inner randomness
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        // Set the position of the note
        note1.set_pos(0);

        let input_note_value_one = JubJubScalar::from(value1);
        // Generate the value commitment of the note from the value and blinder
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        // Assign the postitions of the notes to a position in the tree
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        // Generate the crossover commitment, C.c(v,b)
        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        // Generate the commitment to the first output note, C.c(v,b)
        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        // Generate the commitment to the second output note, C.c(v,b)
        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        // Assign the fee
        let fee = BlsScalar::from(200);

        let mut circuit = build_execute_circuit(
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk2)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            // This is where the wrong values are inputted
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    // This circuit sets the wrong value for the second note,
    // which will fail the commitment check and create
    // an incorrect note. This is preventing the user from
    // falsifying their note value.
    fn test_wrong_note_value_two() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);

        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        // Assign an incorrect value to the note. This will fail in the commitment check and the balance check
        let value2 = 800u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        let fee = BlsScalar::from(200);

        let mut circuit = build_execute_circuit(
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk2)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            // This is where the incorrect values is assigned to the circuit
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    // This circuit tests to see if a wrong nullifier
    // leads to a failed circuit
    fn test_wrong_nullifier() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);

        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        let fee = BlsScalar::from(200);

        // The vector entries for the nulllifier are incorrect
        let mut circuit = build_execute_circuit(
            // Here the second nullifier is declared incorrectly
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk1)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        // Assert the test fails
        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    // The fee is a public input and is the value
    // paid for processing a transaction. With an
    // incorrect value for PI, the test should fail.
    fn test_wrong_fee() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);

        let input_note_value_one = JubJubScalar::from(value1);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        // Assign a wrong fee so the amount paid and balance gadget check is incorrect
        let fee = BlsScalar::from(20);

        let mut circuit = build_execute_circuit(
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk2)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            // Here the incorrect fee is added
            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        // Assert that the proof will fail
        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    // This test pushes the position of the note,
    // after the note position is pushed to the tree.
    // This should fail meaning the user cannot amend
    // the position of the note in the tree after its
    // set.
    fn test_pushing_note_to_wrong_position() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let value1 = 600u64;
        let input_note_blinder_one = JubJubScalar::from(value1);
        let mut note1 = circuit_note(ssk1, value1, 0, input_note_blinder_one);
        note1.set_pos(0);

        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = compute_value_commitment(
            input_note_value_one,
            input_note_blinder_one,
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let value2 = 200u64;
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = circuit_note(ssk2, value2, 0, input_note_blinder_two);
        note2.set_pos(1);

        let input_note_value_two = JubJubScalar::from(value2);
        let input_commitment_two = compute_value_commitment(
            input_note_value_two,
            input_note_blinder_two,
        );

        let mut tree =
            PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        // After the note has been pushed to the tree, set the position elsewhere,
        // this will cause the the preimage and nullifier to fail
        note2.set_pos(5);

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = compute_value_commitment(
            crossover_commitment_value,
            crossover_commitment_blinder,
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = compute_value_commitment(
            obfuscated_note_value_one,
            obfuscated_note_blinder_one,
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = compute_value_commitment(
            obfuscated_note_value_two,
            obfuscated_note_blinder_two,
        );

        let fee = BlsScalar::from(200);

        let mut circuit = build_execute_circuit(
            vec![note1.gen_nullifier(&ssk1), note2.gen_nullifier(&ssk2)],
            vec![note1.hash(), note2.hash()],
            vec![BlsScalar::from(note1.pos()), BlsScalar::from(note2.pos())],
            vec![
                tree.poseidon_branch(tree_pos_1)?.unwrap(),
                tree.poseidon_branch(tree_pos_2)?.unwrap(),
            ],
            vec![
                ssk1.sk_r(note1.stealth_address()),
                ssk2.sk_r(note2.stealth_address()),
            ],
            vec![
                AffinePoint::from(note1.stealth_address().pk_r()),
                AffinePoint::from(note2.stealth_address().pk_r()),
            ],
            vec![input_commitment_one, input_commitment_two],
            vec![input_note_value_one.into(), input_note_value_two.into()],
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()],
            crossover_commitment,
            crossover_commitment_value.into(),
            crossover_commitment_blinder.into(),
            vec![obfuscated_commitment_one, obfuscated_commitment_two],
            vec![
                obfuscated_note_value_one.into(),
                obfuscated_note_value_two.into(),
            ],
            vec![
                obfuscated_note_blinder_one.into(),
                obfuscated_note_blinder_two.into(),
            ],
            fee,
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        add_circuit_public_inputs(&circuit, crossover_commitment, fee, &mut pi);

        // Assert the proof will fail
        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }
}
