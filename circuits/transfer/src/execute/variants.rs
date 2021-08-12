// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{
    CircuitCrossover, CircuitInput, CircuitOutput, ExecuteCircuit,
    POSEIDON_BRANCH_DEPTH,
};
use crate::{gadgets, Error};

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::sponge;
use dusk_poseidon::tree::{self, PoseidonBranch};
use dusk_schnorr::Proof as SchnorrProof;
use phoenix_core::{Crossover, Fee, Note};
use rand_core::{CryptoRng, RngCore};

use std::convert::TryFrom;

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
            pub fn new(
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

            pub fn compute_tx_hash(&mut self) -> &BlsScalar {
                let hash = [self.anchor()]
                    .iter()
                    .chain(self.inputs.iter().map(|input| input.nullifier()))
                    .chain(
                        self.crossover
                            .value_commitment()
                            .to_hash_inputs()
                            .iter(),
                    )
                    .chain([self.crossover.fee().into()].iter())
                    .copied()
                    .chain(self.outputs.iter().flat_map(|output| {
                        output.value_commitment().to_hash_inputs()
                    }))
                    .collect::<Vec<BlsScalar>>();

                self.tx_hash = sponge::hash(hash.as_slice());

                &self.tx_hash
            }

            pub fn compute_signatures<R: RngCore + CryptoRng>(
                &mut self,
                rng: &mut R,
            ) {
                let tx_hash = *self.compute_tx_hash();

                self.inputs.iter_mut().for_each(|input| {
                    let signature = ExecuteCircuit::sign(
                        rng,
                        input.ssk(),
                        input.note(),
                        tx_hash,
                    );

                    input.set_signature(signature);
                });
            }

            pub fn add_input(
                &mut self,
                ssk: SecretSpendKey,
                note: Note,
                branch: PoseidonBranch<POSEIDON_BRANCH_DEPTH>,
                signature: Option<SchnorrProof>,
            ) -> Result<(), Error> {
                let vk = ssk.view_key();

                let value = note.value(Some(&vk))?;
                let blinding_factor = note.blinding_factor(Some(&vk))?;
                let nullifier = note.gen_nullifier(&ssk);

                let input = CircuitInput::new(
                    signature,
                    branch,
                    ssk,
                    note,
                    value,
                    blinding_factor,
                    nullifier,
                );

                self.inputs.push(input);

                Ok(())
            }

            pub fn set_fee(&mut self, fee: &Fee) -> Result<(), Error> {
                let value = 0;
                let blinding_factor = JubJubScalar::zero();
                let value_commitment = (GENERATOR_EXTENDED
                    * JubJubScalar::zero())
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
            ) -> Result<(), Error> {
                let shared_secret = fee.stealth_address().R() * vk.a();
                let shared_secret = shared_secret.into();
                let nonce = BlsScalar::from(*crossover.nonce());

                let data: [BlsScalar; PoseidonCipher::capacity()] = crossover
                    .encrypted_data()
                    .decrypt(&shared_secret, &nonce)?;

                let value = data[0].reduce();
                let value = value.0[0];

                let blinding_factor =
                    JubJubScalar::from_bytes(&data[1].to_bytes())?;
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

            pub fn public_inputs(&self) -> Vec<PublicInputValue> {
                // 1.c opening(A,io,ih)
                let mut pi = vec![self.anchor().into()];

                // 1.f N == H(k',ip)
                let nullifiers = self
                    .inputs
                    .iter()
                    .map(CircuitInput::nullifier)
                    .cloned()
                    .map(|i| i.into());

                pi.extend(nullifiers);

                // 2. commitment(C,cv,cb)
                let crossover =
                    JubJubAffine::from(self.crossover.value_commitment());
                pi.push(crossover.into());

                pi.push(BlsScalar::from(self.crossover.fee()).into());

                // 4.a commitment(V,ov,ob)
                let outputs = self.outputs.iter().map(|output| {
                    JubJubAffine::from(output.note().value_commitment()).into()
                });
                pi.extend(outputs);

                // Transaction hash
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
                composer: &mut StandardComposer,
            ) -> Result<(), PlonkError> {
                let _ = $i::CIRCUIT_ID;

                // Set the common root/anchor for all inputs
                let tx_hash = composer.add_input(self.tx_hash);
                let anchor_s = self.anchor();
                let anchor = composer.add_input(anchor_s);
                composer.constrain_to_constant(
                    anchor,
                    BlsScalar::zero(),
                    Some(-anchor_s),
                );

                let inputs = self
                    .inputs
                    .iter()
                    .try_fold::<_, _, Result<Variable, Error>>(
                        composer.zero_var(),
                        |sum, input| {
                            let witness = input.to_witness(composer)?;

                            // 1.a k := is · G
                            // 1.b k':= is · G∗
                            let k = witness.pk_r;
                            let k_p = witness.pk_r_prime;

                            // 1.c opening(A,io,ih)
                            let anchor_p =
                                tree::merkle_opening(composer, input.branch());
                            composer.assert_equal(anchor, anchor_p);

                            // 1.d ih == H(it,ic,in,k,ir,ip,iψ)
                            let hash = witness.to_hash_inputs();
                            let hash = sponge::gadget(composer, &hash);
                            composer.assert_equal(witness.note_hash, hash);

                            // 1.e doubleSchnorr(iσ,k,k',T)
                            dusk_schnorr::gadgets::double_key_verify(
                                composer,
                                witness.schnorr_r,
                                witness.schnorr_r_prime,
                                witness.schnorr_u,
                                k,
                                k_p,
                                tx_hash,
                            );

                            // 1.f N == H(k',ip)
                            let nullifier = sponge::gadget(
                                composer,
                                &[*k_p.x(), *k_p.y(), witness.pos],
                            );
                            composer.constrain_to_constant(
                                nullifier,
                                BlsScalar::zero(),
                                Some(-input.nullifier()),
                            );

                            // 1.g commitment(ic,iv,ib)
                            let commitment = gadgets::commitment(
                                composer,
                                witness.value,
                                witness.blinding_factor,
                            );

                            // 1.h range(iv,64)
                            composer.range_gate(witness.value, 64);

                            Ok(composer.add(
                                (BlsScalar::one(), sum),
                                (BlsScalar::one(), witness.value),
                                BlsScalar::zero(),
                                None,
                            ))
                        },
                    )
                    .or(Err(PlonkError::CircuitInputsNotFound))?;

                // 2. commitment(C,cv,cb)
                let crossover = self.crossover.to_witness(composer);

                let commitment = gadgets::commitment(
                    composer,
                    crossover.value,
                    crossover.blinding_factor,
                );

                composer.assert_equal_public_point(
                    commitment,
                    self.crossover.value_commitment().into(),
                );

                composer.constrain_to_constant(
                    crossover.fee_value_witness,
                    BlsScalar::zero(),
                    Some(-crossover.fee_value),
                );

                // 3. range(cv,64)
                composer.range_gate(crossover.value, 64);

                let outputs = self.outputs.iter().fold(
                    composer.zero_var(),
                    |sum, output| {
                        let witness = output.to_witness(composer);

                        // 4.a commitment(V,ov,ob)
                        let commitment = gadgets::commitment(
                            composer,
                            witness.value,
                            witness.blinding_factor,
                        );

                        // 4.b range(ov,64)
                        composer.range_gate(witness.value, 64);

                        composer.assert_equal_public_point(
                            commitment,
                            witness.value_commitment.into(),
                        );

                        composer.add(
                            (BlsScalar::one(), sum),
                            (BlsScalar::one(), witness.value),
                            BlsScalar::zero(),
                            None,
                        )
                    },
                );

                // 5. ∑(iv ∈ I) − ∑(ov ∈ O) − cv − F = 0
                let fee_crossover = composer.add(
                    (BlsScalar::one(), crossover.value),
                    (BlsScalar::one(), crossover.fee_value_witness),
                    BlsScalar::zero(),
                    None,
                );

                composer.poly_gate(
                    inputs,
                    outputs,
                    fee_crossover,
                    BlsScalar::zero(),
                    BlsScalar::one(),
                    -BlsScalar::one(),
                    -BlsScalar::one(),
                    BlsScalar::zero(),
                    None,
                );

                // 12. Verify the transaction hash
                composer.constrain_to_constant(
                    tx_hash,
                    BlsScalar::zero(),
                    Some(-self.tx_hash),
                );

                Ok(())
            }

            fn padded_circuit_size(&self) -> usize {
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
