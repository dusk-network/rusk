// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets;

use crossover::CircuitCrossover;
use input::{CircuitInput, WitnessInput};
use output::{CircuitOutput, WitnessOutput};

use anyhow::{anyhow, Result};
use canonical::Store;
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, SecretKey, SecretSpendKey, ViewKey};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_poseidon::tree::{
    self, PoseidonBranch, PoseidonLeaf, PoseidonTree, PoseidonTreeAnnotation,
};
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};
use schnorr::Proof as SchnorrProof;

use dusk_plonk::prelude::*;

mod crossover;
mod input;
mod output;

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
#[derive(Debug, Default, Clone)]
pub struct ExecuteCircuit {
    inputs: Vec<CircuitInput>,
    crossover: CircuitCrossover,
    outputs: Vec<CircuitOutput>,
    tx_hash: BlsScalar,
}

impl ExecuteCircuit {
    pub fn rusk_keys_id(&self) -> &'static str {
        match (self.inputs.len(), self.outputs.len()) {
            (1, 0) => "transfer-execute-1-0",
            (1, 1) => "transfer-execute-1-1",
            (1, 2) => "transfer-execute-1-2",
            (2, 0) => "transfer-execute-2-0",
            (2, 1) => "transfer-execute-2-1",
            (2, 2) => "transfer-execute-2-2",
            (3, 0) => "transfer-execute-3-0",
            (3, 1) => "transfer-execute-3-1",
            (3, 2) => "transfer-execute-3-2",
            (4, 0) => "transfer-execute-4-0",
            (4, 1) => "transfer-execute-4-1",
            (4, 2) => "transfer-execute-4-2",
            _ => unimplemented!(),
        }
    }

    pub fn set_tx_hash(&mut self, tx_hash: BlsScalar) {
        self.tx_hash = tx_hash;
    }

    pub fn sign<R: RngCore + CryptoRng>(
        rng: &mut R,
        ssk: &SecretSpendKey,
        note: &Note,
    ) -> SchnorrProof {
        let message = SIGN_MESSAGE;
        let sk_r = ssk.sk_r(note.stealth_address()).as_ref().clone();
        let secret = SecretKey::from(&sk_r);

        SchnorrProof::new(&secret, rng, message)
    }

    pub fn add_input(
        &mut self,
        ssk: &SecretSpendKey,
        note: Note,
        branch: PoseidonBranch<{ input::POSEIDON_BRANCH_DEPTH }>,
        signature: SchnorrProof,
    ) -> Result<()> {
        let vk = ssk.view_key();

        let value = note
            .value(Some(&vk))
            .map_err(|e| anyhow!("Failed to decrypt value: {:?}", e))?;
        let blinding_factor = note.blinding_factor(Some(&vk)).map_err(|e| {
            anyhow!("Failed to decrypt blinding factor: {:?}", e)
        })?;
        let sk_r = ssk.sk_r(note.stealth_address()).as_ref().clone();
        let nullifier = note.gen_nullifier(&ssk);

        let input = CircuitInput::new(
            signature,
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

    pub fn add_input_from_tree<S, L, A>(
        &mut self,
        ssk: &SecretSpendKey,
        tree: &PoseidonTree<L, A, S, { input::POSEIDON_BRANCH_DEPTH }>,
        pos: usize,
        signature: SchnorrProof,
    ) -> Result<()>
    where
        S: Store,
        L: PoseidonLeaf<S> + Into<Note>,
        A: PoseidonTreeAnnotation<L, S>,
    {
        let note = tree
            .get(pos)
            .map_err(|e| anyhow!("Failed to fetch note from the tree: {}", e))?
            .map(|n| n.into())
            .ok_or(anyhow!("Note not found in the tree after push!"))?;

        let branch = tree
            .branch(pos)
            .map_err(|e| anyhow!("Failed to get the branch: {}", e))?
            .ok_or(anyhow!("Failed to fetch the branch from the tree"))?;

        self.add_input(ssk, note, branch, signature)
    }

    pub fn set_fee(&mut self, fee: &Fee) -> Result<()> {
        let value = 0;
        let blinding_factor = JubJubScalar::zero();
        let value_commitment = (GENERATOR_EXTENDED * JubJubScalar::zero())
            + (GENERATOR_NUMS_EXTENDED * blinding_factor);

        let fee = fee.gas_limit;
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
        vk: &ViewKey,
    ) -> Result<()> {
        let shared_secret = fee.stealth_address().R() * vk.a();
        let shared_secret = shared_secret.into();
        let nonce = BlsScalar::from(*crossover.nonce());

        let data: [BlsScalar; PoseidonCipher::capacity()] = crossover
            .encrypted_data()
            .decrypt(&shared_secret, &nonce)
            .map_err(|e| anyhow!("Failed to decrypt crossover: {:?}", e))?;

        let value = data[0].reduce();
        let value = value.0[0];

        let blinding_factor = JubJubScalar::from_bytes(&data[1].to_bytes())
            .map_err(|e| anyhow!("Failed to convert bls to jubjub: {:?}", e))?;
        let value_commitment = *crossover.value_commitment();

        let fee = fee.gas_limit;
        self.crossover = CircuitCrossover::new(
            value_commitment,
            value,
            blinding_factor,
            fee,
        );

        Ok(())
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

    pub fn add_output(
        &mut self,
        note: Note,
        vk: Option<&ViewKey>,
    ) -> Result<()> {
        let value = note
            .value(vk)
            .map_err(|e| anyhow!("Failed to decrypt value: {:?}", e))?;
        let blinding_factor = note.blinding_factor(vk).map_err(|e| {
            anyhow!("Failed to decrypt blinding factor: {:?}", e)
        })?;

        self.add_output_with_data(note, value, blinding_factor);

        Ok(())
    }

    pub fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut pi = vec![];

        // step 1
        let root = self
            .inputs
            .first()
            .map(|i| *i.branch().root())
            .unwrap_or_default();
        pi.push(root.into());

        // step 4
        pi.extend(
            self.inputs
                .iter()
                .map(|input| input.nullifier().clone().into()),
        );

        // step 7
        pi.push(BlsScalar::from(self.crossover.fee()).into());

        let crossover_value_commitment =
            JubJubAffine::from(self.crossover.value_commitment());
        pi.push(crossover_value_commitment.into());

        // step 9
        pi.extend(self.outputs.iter().map(|output| {
            JubJubAffine::from(output.note().value_commitment()).into()
        }));

        // step 12
        pi.push(self.tx_hash.into());

        pi
    }

    pub fn inputs(&self) -> &[CircuitInput] {
        self.inputs.as_slice()
    }

    pub fn outputs(&self) -> &[CircuitOutput] {
        self.outputs.as_slice()
    }
}

impl Circuit for ExecuteCircuit {
    // TODO Define ID
    const CIRCUIT_ID: [u8; 32] = [0xff; 32];

    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<(), PlonkError> {
        let mut base_root = None;

        // 1. Prove the knowledge of the input Note paths to Note Tree, via root
        // anchor
        let inputs: Vec<WitnessInput> = self
            .inputs
            .iter()
            .map(|input| {
                let branch = input.branch();
                let note = input.to_witness(composer);

                let root_p = tree::merkle_opening(composer, branch);

                // Test the public input only for the first root
                //
                // The remainder roots must be equal to the first (root is
                // unique per proof)
                match base_root {
                    None => {
                        let root = *branch.root();

                        composer.constrain_to_constant(
                            root_p,
                            BlsScalar::zero(),
                            Some(-root),
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
            schnorr::gadgets::double_key_verify(
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

            composer.constrain_to_constant(
                nullifier_p,
                BlsScalar::zero(),
                Some(-nullifier),
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
            composer.constrain_to_constant(
                crossover.fee_value_witness,
                BlsScalar::zero(),
                Some(-crossover.fee_value),
            );

            // value commitment public input
            let value_commitment = crossover.value_commitment.into();
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
        // crossover_value - fee_value = 0
        {
            let zero =
                composer.add_witness_to_circuit_description(BlsScalar::zero());

            let inputs_sum = inputs.iter().fold(zero, |sum, input| {
                composer.add(
                    (BlsScalar::one(), sum),
                    (BlsScalar::one(), input.value),
                    BlsScalar::zero(),
                    None,
                )
            });

            let outputs_sum = outputs.iter().fold(zero, |sum, output| {
                composer.add(
                    (BlsScalar::one(), sum),
                    (BlsScalar::one(), output.value),
                    BlsScalar::zero(),
                    None,
                )
            });

            let fee_crossover = composer.add(
                (BlsScalar::one(), crossover.value),
                (BlsScalar::one(), crossover.fee_value_witness),
                BlsScalar::zero(),
                None,
            );

            composer.poly_gate(
                inputs_sum,
                outputs_sum,
                fee_crossover,
                BlsScalar::zero(),
                BlsScalar::one(),
                -BlsScalar::one(),
                -BlsScalar::one(),
                BlsScalar::zero(),
                None,
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
        composer.constrain_to_constant(
            tx_hash,
            BlsScalar::zero(),
            Some(-self.tx_hash),
        );

        Ok(())
    }

    fn padded_circuit_size(&self) -> usize {
        match (self.inputs.len(), self.outputs.len()) {
            (1, o) if o < 2 => 1 << 15,
            (1, _) | (2, _) => 1 << 16,
            _ => 1 << 17,
        }
    }
}
