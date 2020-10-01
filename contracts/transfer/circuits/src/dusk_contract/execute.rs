// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
use crate::gadgets::secret_key::sk_knowledge;
use crate::gadgets::{
    merkle::merkle, nullifier::nullifier_gadget, preimage::input_preimage,
    range::range,
};
use anyhow::{Error, Result};
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::BlsScalar;
use dusk_plonk::prelude::JubJubScalar;

use dusk_plonk::prelude::*;
use phoenix_core::Note;
use plonk_gadgets::AllocatedScalar;
use poseidon252::sponge::sponge::{sponge_hash, sponge_hash_gadget};
use poseidon252::{PoseidonBranch, PoseidonTree, PoseidonAnnotation};
use kelvin::Blake2b;
use std::ptr::null;

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Default, Clone)]
pub struct ExecuteCircuit {
    /// Storage height of the tree
    // pub anchor: Option<BlsScalar>,
    /// Nullifier for note
    pub nullifiers: Option<Vec<BlsScalar>>,
    /// Note hashes
    pub note_hashes: Option<Vec<BlsScalar>>,
    /// Positions of notes
    pub position_of_notes: Option<Vec<BlsScalar>>,
    /// Poseidon branches of the input notes
    pub input_poseidon_branches: Option<Vec<PoseidonBranch>>,
    /// Input notes secret keys
    pub input_notes_sk: Option<Vec<JubJubScalar>>,
    /// Input notes public keys
    pub input_notes_pk: Option<Vec<AffinePoint>>,
    /// Input commitment points
    pub input_commitments: Option<Vec<AffinePoint>>,
    /// Input note values
    pub input_values: Option<Vec<BlsScalar>>,
    /// Input notes blinders
    pub input_blinders: Option<Vec<BlsScalar>>,
    /// Commitment point to crossover
    pub crossover_commitment: Option<AffinePoint>,
    /// Crossover commitment value
    pub crossover_commitment_value: Option<BlsScalar>,
    /// Crossover commitment blinder
    pub crossover_commitment_blinder: Option<BlsScalar>,
    /// Obfuscated note commitments
    pub obfuscated_commitment_points: Option<Vec<AffinePoint>>,
    /// Obfuscated note values
    pub obfuscated_note_values: Option<Vec<BlsScalar>>,
    /// Obfuscated note blinder
    pub obfuscated_note_blinders: Option<Vec<BlsScalar>>,
    /// Fee
    pub fee: Option<BlsScalar>,
    /// Returns circuit size
    pub size: usize,
    /// Gives Public Inputs
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for ExecuteCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<Vec<PublicInput>, Error> {
        let mut pi: Vec<PublicInput> = vec![];
        // let anchor = self
        //     .anchor
        //     .as_ref()
        //     .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let nullifiers = self
            .nullifiers
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let note_hashes: Vec<AllocatedScalar> = self
            .note_hashes
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|note_hash| AllocatedScalar::allocate(composer, *note_hash))
            .collect();
        let position_of_notes: Vec<AllocatedScalar> = self
            .position_of_notes
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|position_of_notes| {
                AllocatedScalar::allocate(composer, *position_of_notes)
            })
            .collect();
        let input_poseidon_branches = self
            .input_poseidon_branches
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let input_notes_sk: Vec<AllocatedScalar> = self
            .input_notes_sk
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|input_notes_sk| {
                AllocatedScalar::allocate(composer, BlsScalar::from(*input_notes_sk))
            })
            .collect();
        let input_notes_pk: Vec<PlonkPoint> = self
            .input_notes_pk
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(
                (|input_notes_pk| {
                    PlonkPoint::from_private_affine(composer, *input_notes_pk)
                }),
            )
            .collect();
        let input_commitments: Vec<PlonkPoint> = self
            .input_commitments
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(
                (|input_commitments| {
                    PlonkPoint::from_private_affine(
                        composer,
                        *input_commitments,
                    )
                }),
            )
            .collect();
        let input_note_values: Vec<AllocatedScalar> = self
            .input_values
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|input_values| {
                AllocatedScalar::allocate(composer, *input_values)
            })
            .collect();
        let input_notes_blinders: Vec<AllocatedScalar> = self
            .input_blinders
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|input_blinders| {
                AllocatedScalar::allocate(composer, *input_blinders)
            })
            .collect();
        let crossover_commitment = self
            .crossover_commitment
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let crossover_commitment_value = self
            .crossover_commitment_value
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let crossover_commitment_blinder = self
            .crossover_commitment_blinder
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let obfuscated_commitment_points = self
            .obfuscated_commitment_points
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let obfuscated_note_values: Vec<AllocatedScalar> = self
            .obfuscated_note_values
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|obfuscated_note_values| {
                AllocatedScalar::allocate(composer, *obfuscated_note_values)
            })
            .collect();
        let obfuscated_note_blinders: Vec<AllocatedScalar> = self
            .obfuscated_note_blinders
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .map(|obfuscated_note_blinders| {
                AllocatedScalar::allocate(composer, *obfuscated_note_blinders)
            })
            .collect();
        let fee = self
            .fee
            .as_ref()
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;

        let crossover_value =
            AllocatedScalar::allocate(composer, *crossover_commitment_value);
        let crossover_blinder =
            AllocatedScalar::allocate(composer, *crossover_commitment_blinder);

        // 1. Prove the knowledge of the input Note paths to Note Tree, via root anchor
        input_poseidon_branches
            .iter()
            .zip(note_hashes.iter())
            .for_each(|(branch, note_hash)| {
                let root = merkle(composer, branch.clone(), *note_hash);

                pi.push(PublicInput::BlsScalar(
                    -branch.root,
                    composer.circuit_size(),
                ));

                composer.constrain_to_constant(
                    root,
                    BlsScalar::zero(),
                    -branch.root,
                );
            });


        // 2. Prove the knowledge of the pre-images of the input Notes

        let mut i = 0;
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
                println!("commitments - {} - {}", i, composer.circuit_size());
                i += 1;
            });

        // 3. Prove the knowledge of the secret keys corresponding to the public keys in input Notes

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
                let computed_nullifier = nullifier_gadget(composer, *note_position, *sk);

                // Push Public nullifiers
                pi.push(PublicInput::BlsScalar(
                    -nullifier,
                    composer.circuit_size(),
                ));

                // Assert generated nullifiers are equal to publicly inputted nullifiers
                composer.constrain_to_constant(computed_nullifier, BlsScalar::zero(), -nullifier);
            
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
            *crossover_commitment,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, *crossover_commitment);

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
        let all_input_values: BlsScalar = input_note_values
            .iter()
            .fold(BlsScalar::zero(), |acc, value| acc + value.scalar);
        let all_output_values: BlsScalar = obfuscated_note_values
            .iter()
            .fold(BlsScalar::zero(), |acc, value| acc + value.scalar)
            + crossover_commitment_value;
        let zero =
            composer.add_witness_to_circuit_description(BlsScalar::zero());
        pi.push(PublicInput::BlsScalar(*fee, composer.circuit_size()));

        let all_output_values =
            AllocatedScalar::allocate(composer, all_output_values);
        let all_input_values =
            AllocatedScalar::allocate(composer, all_input_values);

        composer.add_gate(
            zero,
            all_output_values.var,
            all_input_values.var,
            BlsScalar::zero(),
            BlsScalar::one(),
            -BlsScalar::one(),
            BlsScalar::zero(),
            *fee,
        );
        
        self.size = composer.circuit_size();
        Ok(pi)
    }

    fn compile(
        &mut self,
        pub_params: &PublicParameters,
    ) -> Result<(ProverKey, VerifierKey, usize), Error> {
        // Setup PublicParams
        let (ck, _) = pub_params.trim(1 << 17)?;
        // Generate & save `ProverKey` with some random values.
        let mut prover = Prover::new(b"TestCircuit");
        // Set size & Pi builder
        self.pi_constructor = Some(self.gadget(prover.mut_cs())?);
        prover.preprocess(&ck)?;

        // Generate & save `VerifierKey` with some random values.
        let mut verifier = Verifier::new(b"TestCircuit");
        self.gadget(verifier.mut_cs())?;
        verifier.preprocess(&ck)?;
        Ok((
            prover
                .prover_key
                .expect("Unexpected error. Missing VerifierKey in compilation")
                .clone(),
            verifier
                .verifier_key
                .expect("Unexpected error. Missing VerifierKey in compilation"),
            self.circuit_size(),
        ))
    }

    fn build_pi(&self, pub_inputs: &[PublicInput]) -> Result<Vec<BlsScalar>> {
        let mut pi = vec![BlsScalar::zero(); self.size];
        self.pi_constructor
            .as_ref()
            .ok_or(CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .enumerate()
            .for_each(|(idx, pi_constr)| {
                match pi_constr {
                    PublicInput::BlsScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::JubJubScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::AffinePoint(_, pos_x, pos_y) => {
                        let (coord_x, coord_y) = (
                            pub_inputs[idx].value()[0],
                            pub_inputs[idx].value()[1],
                        );
                        pi[*pos_x] = -coord_x;
                        pi[*pos_y] = -coord_y;
                    }
                };
            });
        Ok(pi)
    }

    fn circuit_size(&self) -> usize {
        self.size
    }

    fn gen_proof(
        &mut self,
        pub_params: &PublicParameters,
        prover_key: &ProverKey,
        transcript_initialisation: &'static [u8],
    ) -> Result<Proof> {
        let (ck, _) = pub_params.trim(1 << 17)?;
        // New Prover instance
        let mut prover = Prover::new(transcript_initialisation);
        // Fill witnesses for Prover
        self.gadget(prover.mut_cs())?;
        // Add ProverKey to Prover
        prover.prover_key = Some(prover_key.clone());
        prover.prove(&ck)
    }

    fn verify_proof(
        &mut self,
        pub_params: &PublicParameters,
        verifier_key: &VerifierKey,
        transcript_initialisation: &'static [u8],
        proof: &Proof,
        pub_inputs: &[PublicInput],
    ) -> Result<(), Error> {
        let (_, vk) = pub_params.trim(1 << 17)?;
        // New Verifier instance
        let mut verifier = Verifier::new(transcript_initialisation);
        // Fill witnesses for Verifier
        self.gadget(verifier.mut_cs())?;
        verifier.verifier_key = Some(*verifier_key);
        verifier.verify(proof, &vk, &self.build_pi(pub_inputs)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadgets::commitment::commitment;
    use anyhow::Result;
    use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use phoenix_core::{Note, NoteType};
    use rand::Rng;

    #[test]
    fn test_execute() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 600u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk2);
        let value2 = 200u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(200);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        circuit.verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
    }

    #[test]
    fn test_wrong_note_value_one() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 400u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk2);
        let value2 = 200u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(200);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_wrong_note_value_two() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 600u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk2);
        let value2 = 800u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(200);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_wrong_nullifier() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 600u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk1);
        let value2 = 200u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(200);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_wrong_fee() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 600u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk2);
        let value2 = 200u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(20);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_pushing_note_to_wrong_position() -> Result<()> {
        let secret1 = JubJubScalar::from(100 as u64);
        let secret2 = JubJubScalar::from(200 as u64);
        let ssk1 = SecretSpendKey::new(secret1, secret2);
        let psk1 = PublicSpendKey::from(ssk1);
        let value1 = 600u64;
        let r1 = JubJubScalar::from(150 as u64);
        let nonce1 = JubJubScalar::from(350 as u64);
        let input_note_blinder_one = JubJubScalar::from(100 as u64);
        let mut note1 = Note::deterministic(NoteType::Transparent, &r1, nonce1, &psk1, value1, input_note_blinder_one);
        let pos1 = note1.set_pos(0);
        let note_hash1 = note1.hash();
        let pos_a = note1.pos();
        let sk_r1 = ssk1.sk_r(note1.stealth_address());
        let pk_r1 = note1.stealth_address().pk_r();
        let nullifier1 = note1.gen_nullifier(&ssk1);
        let input_note_value_one = JubJubScalar::from(600 as u64);
        let input_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_one),
        );

        let secret3 = JubJubScalar::from(300 as u64);
        let secret4 = JubJubScalar::from(400 as u64);
        let ssk2 = SecretSpendKey::new(secret3, secret4);
        let psk2 = PublicSpendKey::from(ssk2);
        let value2 = 200u64;
        let r2 = JubJubScalar::from(450 as u64);
        let nonce2 = JubJubScalar::from(6750 as u64);
        let input_note_blinder_two = JubJubScalar::from(200 as u64);
        let mut note2 = Note::deterministic(NoteType::Transparent, &r2, nonce2, &psk2, value2, input_note_blinder_two);
        let pos2 = note2.set_pos(1);
        let note_hash2 = note2.hash();
        let pos_b = note2.pos();
        let sk_r2 = ssk2.sk_r(note2.stealth_address());
        let pk_r2 = note2.stealth_address().pk_r().clone();
        let nullifier2 = note2.gen_nullifier(&ssk2);
        let input_note_value_two = JubJubScalar::from(200 as u64);
        let input_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * input_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * input_note_blinder_two),
        );
        let mut tree = PoseidonTree::<Note, PoseidonAnnotation, Blake2b>::new(17);
        let tree_pos_1 = tree.push(note1)?;
        let tree_pos_2 = tree.push(note2)?;

        note2.set_pos(5);
        let pos_b = note2.pos();
        
        let crossover_commitment_value = JubJubScalar::from(200 as u64);
        let crossover_commitment_blinder = JubJubScalar::from(100 as u64);
        let crossover_commitment = AffinePoint::from(
            &(GENERATOR_EXTENDED * crossover_commitment_value)
                + &(GENERATOR_NUMS_EXTENDED * crossover_commitment_blinder),
        );

        let obfuscated_note_value_one = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_one = JubJubScalar::from(100 as u64);
        let obfuscated_commitment_one = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_one)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_one),
        );

        let obfuscated_note_value_two = JubJubScalar::from(200 as u64);
        let obfuscated_note_blinder_two = JubJubScalar::from(200 as u64);
        let obfuscated_commitment_two = AffinePoint::from(
            &(GENERATOR_EXTENDED * obfuscated_note_value_two)
                + &(GENERATOR_NUMS_EXTENDED * obfuscated_note_blinder_two),
        );

        let fee = BlsScalar::from(200);

        let nullifiers = vec![nullifier1, nullifier2];
        let input_positions =
            vec![BlsScalar::from(pos_a), BlsScalar::from(pos_b)];
        let input_poseidon_branches = vec![tree.poseidon_branch(tree_pos_1)?.unwrap(), tree.poseidon_branch(tree_pos_2)?.unwrap()];
        let note_hashes = vec![note_hash1, note_hash2];
        let input_notes_sk = vec![sk_r1, sk_r2];
        let input_notes_pk =
            vec![AffinePoint::from(pk_r1), AffinePoint::from(pk_r2)];
        let input_commitments =
            vec![input_commitment_one, input_commitment_two];
        let input_values =
            vec![input_note_value_one.into(), input_note_value_two.into()];
        let input_blinders =
            vec![input_note_blinder_one.into(), input_note_blinder_two.into()];
        let obfuscated_commitment_points =
            vec![obfuscated_commitment_one, obfuscated_commitment_two];
        let obfuscated_note_values = vec![
            obfuscated_note_value_one.into(),
            obfuscated_note_value_two.into(),
        ];
        let obfuscated_note_blinders = vec![
            obfuscated_note_blinder_one.into(),
            obfuscated_note_blinder_two.into(),
        ];

        let mut circuit = ExecuteCircuit {
            // anchor: None,
            nullifiers: Some(nullifiers),
            note_hashes: Some(note_hashes),
            position_of_notes: Some(input_positions),
            input_poseidon_branches: Some(input_poseidon_branches),
            input_notes_sk: Some(input_notes_sk),
            input_notes_pk: Some(input_notes_pk),
            input_commitments: Some(input_commitments),
            input_values: Some(input_values),
            input_blinders: Some(input_blinders),
            crossover_commitment: Some(crossover_commitment),
            crossover_commitment_value: Some(crossover_commitment_value.into()),
            crossover_commitment_blinder: Some(
                crossover_commitment_blinder.into(),
            ),
            obfuscated_commitment_points: Some(obfuscated_commitment_points),
            obfuscated_note_values: Some(obfuscated_note_values),
            obfuscated_note_blinders: Some(obfuscated_note_blinders),
            fee: Some(fee),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 18, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"Execute")?;

        let mut pi = vec![];
        circuit
            .input_poseidon_branches
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|branch| {
                pi.push(PublicInput::BlsScalar(-branch.root, 0));
            });
        circuit
            .nullifiers
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|nullifier| {
                pi.push(PublicInput::BlsScalar(-nullifier, 0));
            });
        pi.push(PublicInput::AffinePoint(crossover_commitment, 0, 0));
        circuit
            .obfuscated_commitment_points
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|commitment_point| {
                pi.push(PublicInput::AffinePoint(*commitment_point, 0, 0));
            });
        pi.push(PublicInput::BlsScalar(fee, 0));

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"Execute", &proof, &pi)
            .is_err());
        Ok(())
    }
}
