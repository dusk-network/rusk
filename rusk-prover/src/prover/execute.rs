// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use execution_core::{
    transfer::phoenix::{
        value_commitment, InputNoteInfo, OutputNoteInfo, Sender, TxCircuit,
        NOTES_TREE_DEPTH,
    },
    JubJubAffine,
};
use rand::{CryptoRng, RngCore};

use crate::prover::fetch_prover;
use crate::UnprovenTransaction;

pub static EXEC_1_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitOneTwo"));

pub static EXEC_2_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitTwoTwo"));

pub static EXEC_3_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitThreeTwo"));

pub static EXEC_4_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("ExecuteCircuitFourTwo"));

fn create_circuit<const I: usize>(
    utx: &UnprovenTransaction,
) -> Result<TxCircuit<NOTES_TREE_DEPTH, I>, ProverError> {
    // Create the `InputNoteInfo`
    let mut tx_input_notes = Vec::with_capacity(utx.inputs().len());
    utx.inputs.iter().for_each(|input| {
        tx_input_notes.push(InputNoteInfo {
            merkle_opening: input.opening,
            note: input.note.clone(),
            note_pk_p: input.npk_prime.into(),
            value: input.value,
            value_blinder: input.value_blinder,
            nullifier: input.nullifier,
            signature: input.sig,
        });
    });
    let tx_input_notes: [InputNoteInfo<NOTES_TREE_DEPTH>; I] = tx_input_notes
        .try_into()
        .expect("the numbers of input-notes should be as expected");

    // Create the `TxOutputNotes`
    let (
        transfer_note,
        transfer_value,
        transfer_value_blinder,
        transfer_sender_blinder,
    ) = &utx.outputs[0];
    let transfer_value_commitment =
        value_commitment(*transfer_value, *transfer_value_blinder);
    let transfer_note_sender_enc = match transfer_note.sender() {
        Sender::Encryption(enc) => enc,
        Sender::ContractInfo(_) => {
            panic!("The sender needs to be an encryption")
        }
    };

    let (
        change_note,
        change_value,
        change_value_blinder,
        change_sender_blinder,
    ) = &utx.outputs[1];
    let change_value_commitment =
        value_commitment(*change_value, *change_value_blinder);
    let change_note_sender_enc = match change_note.sender() {
        Sender::Encryption(enc) => enc,
        Sender::ContractInfo(_) => {
            panic!("The sender needs to be an encryption")
        }
    };
    let tx_output_notes = [
        OutputNoteInfo {
            value: *transfer_value,
            value_commitment: transfer_value_commitment,
            value_blinder: *transfer_value_blinder,
            note_pk: JubJubAffine::from(
                transfer_note.stealth_address().note_pk().as_ref(),
            ),
            sender_enc: *transfer_note_sender_enc,
            sender_blinder: *transfer_sender_blinder,
        },
        OutputNoteInfo {
            value: *change_value,
            value_commitment: change_value_commitment,
            value_blinder: *change_value_blinder,
            note_pk: JubJubAffine::from(
                change_note.stealth_address().note_pk().as_ref(),
            ),
            sender_enc: *change_note_sender_enc,
            sender_blinder: *change_sender_blinder,
        },
    ];

    // Build the circuit
    let circuit: TxCircuit<NOTES_TREE_DEPTH, I> = TxCircuit {
        input_notes_info: tx_input_notes,
        output_notes_info: tx_output_notes,
        payload_hash: utx.payload_hash(),
        root: utx.payload.tx_skeleton.root,
        deposit: utx.payload.tx_skeleton.deposit,
        max_fee: utx.payload.fee.max_fee(),
        sender_pk: utx.sender_pk,
        signatures: utx.signatures,
    };

    Ok(circuit)
}

impl LocalProver {
    pub(crate) fn local_prove_execute(
        &self,
        circuit_inputs: &[u8],
    ) -> Result<Vec<u8>, ProverError> {
        let utx = UnprovenTransaction::from_slice(circuit_inputs)
            .map_err(|e| ProverError::invalid_data("utx", e))?;

        #[cfg(not(feature = "no_random"))]
        let rng = &mut OsRng;

        #[cfg(feature = "no_random")]
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        match utx.inputs().len() {
            1 => local_prove_exec_1_2(&utx, rng),
            2 => local_prove_exec_2_2(&utx, rng),
            3 => local_prove_exec_3_2(&utx, rng),
            4 => local_prove_exec_4_2(&utx, rng),
            _ => Err(ProverError::from(format!(
                "Invalid I/O count: {}/{}",
                utx.inputs().len(),
                utx.outputs().len()
            ))),
        }
    }
}

fn local_prove_exec_1_2<R>(
    utx: &UnprovenTransaction,
    rng: &mut R,
) -> Result<Vec<u8>, ProverError>
where
    R: RngCore + CryptoRng,
{
    let circuit = create_circuit::<1>(utx)?;

    let (proof, _) = EXEC_1_2_PROVER.prove(rng, &circuit).map_err(|e| {
        ProverError::with_context("Failed proving the circuit", e)
    })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_2_2<R>(
    utx: &UnprovenTransaction,
    rng: &mut R,
) -> Result<Vec<u8>, ProverError>
where
    R: RngCore + CryptoRng,
{
    let circuit = create_circuit::<2>(utx)?;

    let (proof, _) = EXEC_2_2_PROVER.prove(rng, &circuit).map_err(|e| {
        ProverError::with_context("Failed proving the circuit", e)
    })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_3_2<R>(
    utx: &UnprovenTransaction,
    rng: &mut R,
) -> Result<Vec<u8>, ProverError>
where
    R: RngCore + CryptoRng,
{
    let circuit = create_circuit::<3>(utx)?;

    let (proof, _) = EXEC_3_2_PROVER.prove(rng, &circuit).map_err(|e| {
        ProverError::with_context("Failed proving the circuit", e)
    })?;
    Ok(proof.to_bytes().to_vec())
}

fn local_prove_exec_4_2<R>(
    utx: &UnprovenTransaction,
    rng: &mut R,
) -> Result<Vec<u8>, ProverError>
where
    R: RngCore + CryptoRng,
{
    let circuit = create_circuit::<4>(utx)?;

    let (proof, _) = EXEC_4_2_PROVER.prove(rng, &circuit).map_err(|e| {
        ProverError::with_context("Failed proving the circuit", e)
    })?;
    Ok(proof.to_bytes().to_vec())
}
