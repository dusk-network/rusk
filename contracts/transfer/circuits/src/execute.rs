// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use anyhow::{anyhow, Result};
use canonical::Store;
use dusk_pki::Ownable;
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base;
use dusk_plonk::constraint_system::ecc::Point as PlonkPoint;
use dusk_plonk::jubjub::{
    JubJubExtended, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use phoenix_core::Note;
use poseidon252::cipher::PoseidonCipher;
use poseidon252::sponge::sponge::sponge_hash_gadget;
use poseidon252::tree::{
    self, PoseidonBranch, PoseidonLeaf, PoseidonTree, PoseidonTreeAnnotation,
};
use rand_core::{CryptoRng, RngCore};
use schnorr::double_key::{SecretKey as SchnorrSecret, Signature};
use schnorr::gadgets as schnorr_gadgets;

use dusk_plonk::prelude::*;

/// The circuit responsible for creating a zero-knowledge proof
/// for a 'send to contract transparent' transaction.
#[derive(Debug, Clone)]
pub struct ExecuteCircuit<const DEPTH: usize> {
    trim_size: usize,
    pub pi_positions: Vec<PublicInput>,
    pub inputs: Vec<CircuitInput<DEPTH>>,
    pub crossover: CircuitCrossover,
    pub outputs: Vec<CircuitOutput>,
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessCrossover {
    pub value_commitment: PlonkPoint,
    pub value: Variable,
    pub blinding_factor: Variable,

    // Public data
    pub fee_value: BlsScalar,
}

/// Coupled code
///
/// Currently, Plonk is not a dependency of phoenix-core. This means the circuit
/// construction of the note must be done here.
///
/// Ideally, there would be a `fn hash_inputs_witness(&self, composer)` implemented for
/// `Note`.
///
/// Since the circuit will perform a pre-image check over the result of this function, the
/// structure is safe
///
/// However, if `Note::hash_inputs` ever change, this circuit will be broken
#[derive(Debug, Clone, Copy)]
pub struct WitnessInput {
    pub sk_r: Variable,
    pub pk_r: PlonkPoint,
    pub note_hash: Variable,

    pub note_type: Variable,
    pub value_commitment: PlonkPoint,
    pub nonce: Variable,
    pub r: PlonkPoint,
    pub pos: Variable,
    pub cipher: [Variable; PoseidonCipher::cipher_size()],

    pub value: Variable,
    pub blinding_factor: Variable,

    pub pk_r_prime: PlonkPoint,
    pub schnorr_message: Variable,
    pub schnorr_u: Variable,
    pub schnorr_r: PlonkPoint,
    pub schnorr_r_prime: PlonkPoint,

    // Public data
    pub nullifier: BlsScalar,
}

impl WitnessInput {
    pub fn to_hash_inputs(&self) -> [Variable; 12] {
        [
            self.note_type,
            *self.value_commitment.x(),
            *self.value_commitment.y(),
            self.nonce,
            *self.pk_r.x(),
            *self.pk_r.y(),
            *self.r.x(),
            *self.r.y(),
            self.pos,
            self.cipher[0],
            self.cipher[1],
            self.cipher[2],
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WitnessOutput {
    pub value_commitment: PlonkPoint,
    pub value: Variable,
    pub blinding_factor: Variable,
}

#[derive(Debug, Default, Clone)]
pub struct CircuitCrossover {
    value_commitment: JubJubExtended,
    value: u64,
    blinding_factor: JubJubScalar,
}

impl CircuitCrossover {
    pub fn new(
        value_commitment: JubJubExtended,
        value: u64,
        blinding_factor: JubJubScalar,
    ) -> Self {
        Self {
            value_commitment,
            value,
            blinding_factor,
        }
    }

    pub fn to_witness(
        &self,
        composer: &mut StandardComposer,
    ) -> WitnessCrossover {
        let value_commitment = self.value_commitment.into();
        let value_commitment =
            PlonkPoint::from_private_affine(composer, value_commitment);

        let value = BlsScalar::from(self.value);
        let fee_value = value;
        let value = composer.add_input(value);

        let blinding_factor = BlsScalar::from(self.blinding_factor);
        let blinding_factor = composer.add_input(blinding_factor);

        WitnessCrossover {
            value_commitment,
            value,
            blinding_factor,
            fee_value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitOutput {
    note: Note,
    value: u64,
    blinding_factor: JubJubScalar,
}

impl CircuitOutput {
    pub fn new(note: Note, value: u64, blinding_factor: JubJubScalar) -> Self {
        Self {
            note,
            value,
            blinding_factor,
        }
    }

    pub fn to_witness(&self, composer: &mut StandardComposer) -> WitnessOutput {
        let value_commitment = self.note.value_commitment().into();
        let value_commitment =
            PlonkPoint::from_private_affine(composer, value_commitment);

        let value = composer.add_input(self.value.into());
        let blinding_factor = composer.add_input(self.blinding_factor.into());

        WitnessOutput {
            value_commitment,
            value,
            blinding_factor,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitInput<const DEPTH: usize> {
    sk_r: JubJubScalar,
    branch: PoseidonBranch<DEPTH>,
    note: Note,
    value: u64,
    blinding_factor: JubJubScalar,
    signature: Signature,
    nullifier: BlsScalar,
}

impl<const DEPTH: usize> CircuitInput<DEPTH> {
    pub fn new<R, L, A, S>(
        rng: &mut R,
        tree: &PoseidonTree<L, A, S, DEPTH>,
        sk_r: JubJubScalar,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
    ) -> Result<Self>
    where
        R: RngCore + CryptoRng,
        L: PoseidonLeaf<S>,
        A: PoseidonTreeAnnotation<L, S>,
        S: Store,
    {
        let branch = tree
            .branch(note.pos() as usize)
            .map_err(|e| anyhow!("Poseidon tree error: {}", e))
            .and_then(|branch| {
                branch.ok_or(anyhow!(
                    "The provided input note doesn't belong to the tree!"
                ))
            })?;

        let message = ExecuteCircuit::<DEPTH>::sign_message();
        let signature = SchnorrSecret::from(&sk_r).sign(rng, message);

        Ok(Self {
            sk_r,
            branch,
            note,
            value,
            blinding_factor,
            signature,
            nullifier,
        })
    }

    pub fn note(&self) -> &Note {
        &self.note
    }

    pub fn sk_r(&self) -> &JubJubScalar {
        &self.sk_r
    }

    pub const fn branch(&self) -> &PoseidonBranch<DEPTH> {
        &self.branch
    }

    pub fn to_witness(&self, composer: &mut StandardComposer) -> WitnessInput {
        let nullifier = self.nullifier;

        let note = self.note();

        // TODO victor - review this conversion, doesn't seem safe
        let sk_r = *self.sk_r();
        let sk_r = composer.add_input(sk_r.into());

        let pk_r = fixed_base::scalar_mul(composer, sk_r, GENERATOR_EXTENDED);
        let pk_r = *pk_r.point();

        let note_hash = note.hash();
        let note_hash = composer.add_input(note_hash);

        let hash_inputs = note.hash_inputs();

        let note_type = hash_inputs[0];
        let note_type = composer.add_input(note_type);

        // Plonk API will not allow points to be constructed from variables
        let value_commitment = note.value_commitment().into();
        let value_commitment =
            PlonkPoint::from_private_affine(composer, value_commitment);

        let nonce = hash_inputs[3];
        let nonce = composer.add_input(nonce);

        let r = note.stealth_address().R().into();
        let r = PlonkPoint::from_private_affine(composer, r);

        let pos = hash_inputs[8];
        let pos = composer.add_input(pos);

        let mut cipher = [pos; 3];
        cipher
            .iter_mut()
            .zip(hash_inputs[9..].iter())
            .for_each(|(c, i)| {
                *c = composer.add_input(*i);
            });

        let value = composer.add_input(self.value.into());
        let blinding_factor = composer.add_input(self.blinding_factor.into());

        let pk_r_prime =
            fixed_base::scalar_mul(composer, sk_r, GENERATOR_NUMS_EXTENDED);
        let pk_r_prime = *pk_r_prime.point();
        let schnorr_message = ExecuteCircuit::<DEPTH>::sign_message();
        let schnorr_message =
            composer.add_witness_to_circuit_description(schnorr_message);
        let schnorr_u = *self.signature.u();
        let schnorr_u = composer.add_input(schnorr_u.into());
        let schnorr_r = self.signature.R().into();
        let schnorr_r = PlonkPoint::from_private_affine(composer, schnorr_r);
        let schnorr_r_prime = self.signature.R_prime().into();
        let schnorr_r_prime =
            PlonkPoint::from_private_affine(composer, schnorr_r_prime);

        WitnessInput {
            sk_r,
            pk_r,
            note_hash,

            note_type,
            value_commitment,
            nonce,
            r,
            pos,
            cipher,

            value,
            blinding_factor,

            pk_r_prime,
            schnorr_message,
            schnorr_u,
            schnorr_r,
            schnorr_r_prime,

            nullifier,
        }
    }
}

impl<const DEPTH: usize> ExecuteCircuit<DEPTH> {
    pub fn with_size(trim_size: usize) -> Self {
        Self {
            trim_size,
            pi_positions: vec![],
            inputs: vec![],
            crossover: Default::default(),
            outputs: vec![],
        }
    }

    pub fn add_input<R, L, A, S>(
        &mut self,
        rng: &mut R,
        tree: &PoseidonTree<L, A, S, DEPTH>,
        sk_r: JubJubScalar,
        note: Note,
        value: u64,
        blinding_factor: JubJubScalar,
        nullifier: BlsScalar,
    ) -> Result<()>
    where
        R: RngCore + CryptoRng,
        L: PoseidonLeaf<S>,
        A: PoseidonTreeAnnotation<L, S>,
        S: Store,
    {
        let input = CircuitInput::new(
            rng,
            tree,
            sk_r,
            note,
            value,
            blinding_factor,
            nullifier,
        )?;

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
    /// The signature is provided outside the circuit; so that's why it is constant
    pub const fn sign_message() -> BlsScalar {
        BlsScalar::one()
    }
}

impl<const DEPTH: usize> Circuit<'_> for ExecuteCircuit<DEPTH> {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<()> {
        let mut pi = vec![];

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

                let root = branch.root();
                pi.push(PublicInput::BlsScalar(root, composer.circuit_size()));
                composer.constrain_to_constant(
                    root_p,
                    BlsScalar::zero(),
                    -root,
                );

                note
            })
            .collect();

        // 2. Prove the knowledge of the pre-images of the input Notes
        inputs.iter().for_each(|input| {
            let note_hash = input.note_hash;
            let hash_inputs = input.to_hash_inputs();

            let note_hash_p = sponge_hash_gadget(composer, &hash_inputs);

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
            )
        });

        // 4. Prove the correctness of the nullifiers
        inputs.iter().for_each(|input| {
            let nullifier = input.nullifier;
            let sk_r = input.sk_r;
            let pos = input.pos;

            let nullifier_p = sponge_hash_gadget(composer, &[sk_r, pos]);

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
            let value_commitment = crossover.value_commitment;
            let value_commitment_p = gadgets::commitment(
                composer,
                crossover.value,
                crossover.blinding_factor,
            );

            composer.assert_equal_point(value_commitment, value_commitment_p);

            pi.push(PublicInput::BlsScalar(
                crossover.fee_value,
                composer.circuit_size(),
            ));
            composer.constrain_to_constant(
                crossover.value,
                BlsScalar::zero(),
                -crossover.fee_value,
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

                let value_commitment = output.value_commitment;
                let value_commitment_p = gadgets::commitment(
                    composer,
                    output.value,
                    output.blinding_factor,
                );

                composer
                    .assert_equal_point(value_commitment, value_commitment_p);

                output
            })
            .collect();

        // 10. Prove that the value of the openings of the commitments of the
        // output Obfuscated Notes is in range
        outputs.iter().for_each(|output| {
            composer.range_gate(output.value, 64);
        });

        // 11. Prove that sum(inputs.value) - sum(outputs.value) - crossover_value = 0
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

        self.get_mut_pi_positions().extend_from_slice(pi.as_slice());

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

    /// /// Return a mutable reference to the Public Inputs storage of the
    /// circuit.
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
    use crate::leaf::NoteLeaf;
    use anyhow::Result;
    use canonical_host::MemStore;
    use dusk_pki::{Ownable, SecretSpendKey};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use phoenix_core::Note;
    use poseidon252::tree::{PoseidonAnnotation, PoseidonTree};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use std::convert::TryInto;

    #[test]
    // This test ensures the execute gadget is done correctly
    // by creating two notes and setting their field values
    // in the execute circuit
    fn test_execute() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(2324u64);

        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

        let mut circuit = ExecuteCircuit::with_size(1 << 16);

        let a_ssk = SecretSpendKey::random(&mut rng);
        let a_psk = a_ssk.public_key();
        let a_value = 600;
        let a_blinding_factor = Note::transparent_blinding_factor();
        let a_note = Note::transparent(&mut rng, &a_psk, a_value);

        let p = tree.push(a_note.into()).expect("Tree append error");
        let a_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");

        let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
        let a_nullifier = a_note.gen_nullifier(&a_ssk);
        circuit.add_input(
            &mut rng,
            &tree,
            a_sk_r,
            a_note,
            a_value,
            a_blinding_factor,
            a_nullifier,
        )?;

        let b_ssk = SecretSpendKey::random(&mut rng);
        let b_psk = b_ssk.public_key();
        let b_value = 450;
        let b_blinding_factor = JubJubScalar::random(&mut rng);
        let b_note =
            Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
        circuit.add_output(b_note, b_value, b_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_key();
        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (_, crossover) = c_note
            .try_into()
            .expect("Failed to generate fee and crossover!");
        let c_value_commitment = *crossover.value_commitment();
        circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

        let d_ssk = SecretSpendKey::random(&mut rng);
        let d_psk = d_ssk.public_key();
        let d_value = 750;
        let d_blinding_factor = Note::transparent_blinding_factor();
        let d_note = Note::transparent(&mut rng, &d_psk, d_value);
        circuit.add_output(d_note, d_value, d_blinding_factor);

        let e_ssk = SecretSpendKey::random(&mut rng);
        let e_psk = e_ssk.public_key();
        let e_value = 700;
        let e_blinding_factor = JubJubScalar::random(&mut rng);
        let e_note =
            Note::obfuscated(&mut rng, &e_psk, e_value, e_blinding_factor);

        let p = tree.push(e_note.into()).expect("Tree append error");
        let e_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");

        let e_sk_r = e_ssk.sk_r(e_note.stealth_address());
        let e_nullifier = e_note.gen_nullifier(&e_ssk);
        circuit.add_input(
            &mut rng,
            &tree,
            e_sk_r,
            e_note,
            e_value,
            e_blinding_factor,
            e_nullifier,
        )?;

        // Generate Composer & Public Parameters
        let pp = PublicParameters::setup(
            circuit.get_trim_size(),
            &mut rand::thread_rng(),
        )?;

        let (pk, vk) = circuit.compile(&pp)?;
        let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
        let pi = circuit.get_pi_positions().clone();

        circuit.verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
    }

    #[test]
    // This test ensures the execute gadget is done correctly
    // by creating two notes and setting their field values
    // in the execute circuit
    fn test_wrong_note_value_one() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(2324u64);

        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

        let mut circuit = ExecuteCircuit::with_size(1 << 15);

        let a_ssk = SecretSpendKey::random(&mut rng);
        let a_psk = a_ssk.public_key();
        let a_value = 600;
        let a_blinding_factor = Note::transparent_blinding_factor();
        let a_note = Note::transparent(&mut rng, &a_psk, a_value);

        let p = tree.push(a_note.into()).expect("Tree append error");
        let a_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");

        let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
        let a_nullifier = a_note.gen_nullifier(&a_ssk);
        circuit.add_input(
            &mut rng,
            &tree,
            a_sk_r,
            a_note,
            a_value,
            a_blinding_factor,
            a_nullifier,
        )?;

        let b_ssk = SecretSpendKey::random(&mut rng);
        let b_psk = b_ssk.public_key();
        let b_value = 150;
        let b_blinding_factor = JubJubScalar::random(&mut rng);
        let b_note =
            Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
        circuit.add_output(b_note, b_value, b_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_key();
        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (_, crossover) = c_note
            .try_into()
            .expect("Failed to generate fee and crossover!");
        let c_value_commitment = *crossover.value_commitment();
        circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

        let d_ssk = SecretSpendKey::random(&mut rng);
        let d_psk = d_ssk.public_key();
        let d_value_note = 351;
        let d_value_circuit = 350;
        let d_blinding_factor = Note::transparent_blinding_factor();
        let d_note = Note::transparent(&mut rng, &d_psk, d_value_note);
        circuit.add_output(d_note, d_value_circuit, d_blinding_factor);

        // Generate Composer & Public Parameters
        let pp = PublicParameters::setup(
            circuit.get_trim_size(),
            &mut rand::thread_rng(),
        )?;

        let (pk, vk) = circuit.compile(&pp)?;
        let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
        let pi = circuit.get_pi_positions().clone();

        let verify = circuit
            .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
            .is_ok();
        assert!(!verify);

        Ok(())
    }

    #[test]
    // This circuit tests to see if a wrong nullifier
    // leads to a failed circuit
    fn test_wrong_nullifier() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(2324u64);

        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

        let mut circuit = ExecuteCircuit::with_size(1 << 15);

        let a_ssk = SecretSpendKey::random(&mut rng);
        let a_psk = a_ssk.public_key();
        let a_value = 600;
        let a_blinding_factor = Note::transparent_blinding_factor();
        let a_note = Note::transparent(&mut rng, &a_psk, a_value);

        let p = tree.push(a_note.into()).expect("Tree append error");
        let a_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");

        let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
        let mut a_nullifier = a_note.gen_nullifier(&a_ssk);
        a_nullifier += BlsScalar::one();
        circuit.add_input(
            &mut rng,
            &tree,
            a_sk_r,
            a_note,
            a_value,
            a_blinding_factor,
            a_nullifier,
        )?;

        let b_ssk = SecretSpendKey::random(&mut rng);
        let b_psk = b_ssk.public_key();
        let b_value = 150;
        let b_blinding_factor = JubJubScalar::random(&mut rng);
        let b_note =
            Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
        circuit.add_output(b_note, b_value, b_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_key();
        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (_, crossover) = c_note
            .try_into()
            .expect("Failed to generate fee and crossover!");
        let c_value_commitment = *crossover.value_commitment();
        circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

        let d_ssk = SecretSpendKey::random(&mut rng);
        let d_psk = d_ssk.public_key();
        let d_value = 350;
        let d_blinding_factor = Note::transparent_blinding_factor();
        let d_note = Note::transparent(&mut rng, &d_psk, d_value);
        circuit.add_output(d_note, d_value, d_blinding_factor);

        // Generate Composer & Public Parameters
        let pp = PublicParameters::setup(
            circuit.get_trim_size(),
            &mut rand::thread_rng(),
        )?;

        let (pk, vk) = circuit.compile(&pp)?;
        let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
        let pi = circuit.get_pi_positions().clone();

        let verify = circuit
            .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
            .is_ok();
        assert!(!verify);

        Ok(())
    }

    #[test]
    // The fee is a public input and is the value
    // paid for processing a transaction. With an
    // incorrect value for PI, the test should fail.
    fn test_wrong_fee() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(2324u64);

        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

        let mut circuit = ExecuteCircuit::with_size(1 << 15);

        let a_ssk = SecretSpendKey::random(&mut rng);
        let a_psk = a_ssk.public_key();
        let a_value = 600;
        let a_blinding_factor = Note::transparent_blinding_factor();
        let a_note = Note::transparent(&mut rng, &a_psk, a_value);

        let p = tree.push(a_note.into()).expect("Tree append error");
        let a_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");

        let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
        let a_nullifier = a_note.gen_nullifier(&a_ssk);
        circuit.add_input(
            &mut rng,
            &tree,
            a_sk_r,
            a_note,
            a_value,
            a_blinding_factor,
            a_nullifier,
        )?;

        let b_ssk = SecretSpendKey::random(&mut rng);
        let b_psk = b_ssk.public_key();
        let b_value = 150;
        let b_blinding_factor = JubJubScalar::random(&mut rng);
        let b_note =
            Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
        circuit.add_output(b_note, b_value, b_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_key();
        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (_, crossover) = c_note
            .try_into()
            .expect("Failed to generate fee and crossover!");
        let c_value_commitment = *crossover.value_commitment();
        circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

        let d_ssk = SecretSpendKey::random(&mut rng);
        let d_psk = d_ssk.public_key();
        let d_value = 350;
        let d_blinding_factor = Note::transparent_blinding_factor();
        let d_note = Note::transparent(&mut rng, &d_psk, d_value);
        circuit.add_output(d_note, d_value, d_blinding_factor);

        // Generate Composer & Public Parameters
        let pp = PublicParameters::setup(
            circuit.get_trim_size(),
            &mut rand::thread_rng(),
        )?;

        let (pk, vk) = circuit.compile(&pp)?;
        circuit.get_mut_pi_positions().clear();

        let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
        let mut pi = circuit.get_pi_positions().clone();

        let fee = BlsScalar::from(c_value);
        match &mut pi[2] {
            PublicInput::BlsScalar(f, _) if f == &fee => *f += BlsScalar::one(),
            _ => panic!("Unexpected public input!"),
        }

        let verify = circuit
            .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
            .is_ok();
        assert!(!verify);

        Ok(())
    }

    #[test]
    // This test pushes the position of the note,
    // after the note position is pushed to the tree.
    // This should fail meaning the user cannot amend
    // the position of the note in the tree after its
    // set.
    fn test_pushing_note_to_wrong_position() -> Result<()> {
        let mut rng = StdRng::seed_from_u64(2324u64);

        let mut tree =
            PoseidonTree::<NoteLeaf, PoseidonAnnotation, MemStore, 17>::new();

        let mut circuit = ExecuteCircuit::with_size(1 << 15);

        let a_ssk = SecretSpendKey::random(&mut rng);
        let a_psk = a_ssk.public_key();
        let a_value = 600;
        let a_blinding_factor = Note::transparent_blinding_factor();
        let a_note = Note::transparent(&mut rng, &a_psk, a_value);

        let p = tree.push(a_note.into()).expect("Tree append error");
        let mut a_note = tree
            .get(p)
            .expect("Tree fetch error")
            .map(|n| Note::from(n))
            .expect("a_note not found!");
        let pos = a_note.pos();
        a_note.set_pos(pos + 1);

        let a_sk_r = a_ssk.sk_r(a_note.stealth_address());
        let a_nullifier = a_note.gen_nullifier(&a_ssk);
        circuit
            .add_input(
                &mut rng,
                &tree,
                a_sk_r,
                a_note,
                a_value,
                a_blinding_factor,
                a_nullifier,
            )
            .unwrap_or(());

        let b_ssk = SecretSpendKey::random(&mut rng);
        let b_psk = b_ssk.public_key();
        let b_value = 150;
        let b_blinding_factor = JubJubScalar::random(&mut rng);
        let b_note =
            Note::obfuscated(&mut rng, &b_psk, b_value, b_blinding_factor);
        circuit.add_output(b_note, b_value, b_blinding_factor);

        let c_ssk = SecretSpendKey::random(&mut rng);
        let c_psk = c_ssk.public_key();
        let c_value = 100;
        let c_blinding_factor = JubJubScalar::random(&mut rng);
        let c_note =
            Note::obfuscated(&mut rng, &c_psk, c_value, c_blinding_factor);
        let (_, crossover) = c_note
            .try_into()
            .expect("Failed to generate fee and crossover!");
        let c_value_commitment = *crossover.value_commitment();
        circuit.set_crossover(c_value_commitment, c_value, c_blinding_factor);

        let d_ssk = SecretSpendKey::random(&mut rng);
        let d_psk = d_ssk.public_key();
        let d_value = 350;
        let d_blinding_factor = Note::transparent_blinding_factor();
        let d_note = Note::transparent(&mut rng, &d_psk, d_value);
        circuit.add_output(d_note, d_value, d_blinding_factor);

        // Generate Composer & Public Parameters
        let pp = PublicParameters::setup(
            circuit.get_trim_size(),
            &mut rand::thread_rng(),
        )?;

        let (pk, vk) = circuit.compile(&pp)?;
        circuit.get_mut_pi_positions().clear();

        let proof = circuit.gen_proof(&pp, &pk, b"Execute")?;
        let pi = circuit.get_pi_positions().clone();

        let verify = circuit
            .verify_proof(&pp, &vk, b"Execute", &proof, pi.as_slice())
            .is_ok();
        assert!(!verify);

        Ok(())
    }
}
