// // This Source Code Form is subject to the terms of the Mozilla Public
// // License, v. 2.0. If a copy of the MPL was not distributed with this
// // file, You can obtain one at http://mozilla.org/MPL/2.0/.
// //
// // Copyright (c) DUSK NETWORK. All rights reserved.

// XXX: THIS GADGET NEEDS REFACTORING WHEN THE GITBOOK SPECS ARE READY

// /// This gadget constructs the circuit for an 'Execute' call on the DUSK token contract.
// pub fn execute_gadget(
//     composer: &mut StandardComposer) {
//     // Define an accumulator which we will use to prove that the sum of all inputs
//     // equals the sum of all outputs.
//     //
//     // Note that we are not using the balance gadget here, since the fee output
//     // needs to be a public input, and the balance gadget only constrains
//     // the fee.
//     let mut sum = composer.zero_var;

//     // Inputs
//     tx.inputs().iter().for_each(|tx_input| {
//         // Merkle opening, preimage knowledge
//         // and nullifier.
//         // TODO: get branch
//         // gadgets::merkle(composer, branch, tx_input);
//         gadgets::input_preimage(composer, tx_input);

//         // TODO: ecc_gate function from PLONK
//         //gadget::secret_key();

//         gadgets::nullifier(composer, tx_input);
//         gadgets::commitment(composer, tx_input);
//         gadgets::range(composer, tx_input);

//         // Constrain the sum of all of the inputs
//         let value = composer.add_input(BlsScalar::from(tx_input.value()));
//         sum = composer.add(
//             (BlsScalar::one(), sum),
//             (BlsScalar::one(), value),
//             BlsScalar::zero(),
//             BlsScalar::zero(),
//         );
//     });

//     // Outputs
//     tx.outputs().iter().for_each(|tx_output| {
//         gadgets::commitment(composer, tx_output);
//         gadgets::range(composer, tx_output);

//         // Constrain the sum of all outputs
//         let value = composer.add_input(BlsScalar::from(tx_output.value()));
//         sum = composer.add(
//             (BlsScalar::one(), sum),
//             (-BlsScalar::one(), value),
//             BlsScalar::zero(),
//             BlsScalar::zero(),
//         );
//     });

//     // Crossover
//     if tx.crossover().is_some() {
//         let crossover = tx.crossover().unwrap();
//         gadgets::commitment(composer, &crossover);
//         gadgets::range(composer, &crossover);
//         let value = composer.add_input(BlsScalar::from(crossover.value()));
//         sum = composer.add(
//             (BlsScalar::one(), sum),
//             (-BlsScalar::one(), value),
//             BlsScalar::zero(),
//             BlsScalar::zero(),
//         );
//     }

//     // Contract output
//     if tx.contract_output().is_some() {
//         let contract_output = tx.contract_output().unwrap();
//         gadgets::commitment(composer, &contract_output);
//         gadgets::range(composer, &contract_output);
//         let value = composer.add_input(BlsScalar::from(contract_output.value()));
//         sum = composer.add(
//             (BlsScalar::one(), sum),
//             (-BlsScalar::one(), value),
//             BlsScalar::zero(),
//             BlsScalar::zero(),
//         );
//     }

//     let fee = *tx.fee();

//     sum = composer.add(
//         (-BlsScalar::one(), sum),
//         (BlsScalar::one(), composer.zero_var),
//         BlsScalar::zero(),
//         BlsScalar::from(fee.value()),
//     );

//     composer.constrain_to_constant(sum, BlsScalar::zero(), BlsScalar::zero());
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         crypto, Note, NoteGenerator, ObfuscatedNote, SecretKey, Transaction, TransparentNote,
//     };
//     use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
//     use dusk_plonk::fft::EvaluationDomain;
//     use merlin::Transcript;

//     #[test]
//     fn test_execute_transparent() {
//         let mut tx = Transaction::default();

//         let sk = SecretKey::default();
//         let pk = sk.public_key();
//         let value = 100;
//         let note = TransparentNote::output(&pk, value).0;
//         let merkle_opening = crypto::MerkleProof::mock(note.hash());
//         tx.push_input(note.to_transaction_input(merkle_opening, sk).unwrap())
//             .unwrap();

//         let sk = SecretKey::default();
//         let pk = sk.public_key();
//         let value = 95;
//         let (note, blinding_factor) = TransparentNote::output(&pk, value);
//         tx.push_output(note.to_transaction_output(value, blinding_factor, pk))
//             .unwrap();

//         let sk = SecretKey::default();
//         let pk = sk.public_key();
//         let value = 2;
//         let (note, blinding_factor) = TransparentNote::output(&pk, value);
//         tx.push_output(note.to_transaction_output(value, blinding_factor, pk))
//             .unwrap();

//         let sk = SecretKey::default();
//         let pk = sk.public_key();
//         let value = 3;
//         let (note, blinding_factor) = TransparentNote::output(&pk, value);
//         tx.set_fee(note.to_transaction_output(value, blinding_factor, pk));

//         let mut composer = StandardComposer::new();

//         execute_gadget(&mut composer, &tx);

//         composer.add_dummy_constraints();

//         // Generate Composer & Public Parameters
//         let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
//         let (ck, vk) = pub_params.trim(1 << 16).unwrap();
//         let mut transcript = Transcript::new(b"TEST");

//         let circuit = composer.preprocess(
//             &ck,
//             &mut transcript,
//             &EvaluationDomain::new(composer.circuit_size()).unwrap(),
//         );

//         let proof = composer.prove(&ck, &circuit, &mut transcript.clone());

//         assert!(proof.verify(&circuit, &mut transcript, &vk, &composer.public_inputs()));
//     }
