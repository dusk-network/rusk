// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use execution_core::{
    transfer::{
        ContractCall, Fee, Payload, Transaction, TreeLeaf, TRANSFER_TREE_DEPTH,
    },
    value_commitment, BlsScalar, JubJubScalar, Note, PublicKey,
    SchnorrSecretKey, SecretKey, Sender, TxSkeleton, ViewKey,
};
use rusk_abi::{ContractError, ContractId, Error, Session, TRANSFER_CONTRACT};

use dusk_bytes::Serializable;
use dusk_plonk::prelude::*;
use ff::Field;
use phoenix_circuits::transaction::{TxCircuit, TxInputNote, TxOutputNote};
use poseidon_merkle::Opening as PoseidonOpening;

use rand::rngs::StdRng;
use rand::SeedableRng;

const POINT_LIMIT: u64 = 0x10_000_000;

pub fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<TreeLeaf>, Error> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_height",
        &height,
        POINT_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn leaves_from_pos(
    session: &mut Session,
    pos: u64,
) -> Result<Vec<TreeLeaf>, Error> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_pos",
        &pos,
        POINT_LIMIT,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

pub fn num_notes(session: &mut Session) -> Result<u64, Error> {
    session
        .call(TRANSFER_CONTRACT, "num_notes", &(), u64::MAX)
        .map(|r| r.data)
}

pub fn update_root(session: &mut Session) -> Result<(), Error> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn root(session: &mut Session) -> Result<BlsScalar, Error> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

pub fn contract_balance(
    session: &mut Session,
    contract: ContractId,
) -> Result<u64, Error> {
    session
        .call(
            TRANSFER_CONTRACT,
            "contract_balance",
            &contract,
            POINT_LIMIT,
        )
        .map(|r| r.data)
}

pub fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH>>, Error> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, POINT_LIMIT)
        .map(|r| r.data)
}

pub fn prover_verifier(input_notes: usize) -> (Prover, Verifier) {
    let circuit_name = match input_notes {
        1 => "ExecuteCircuitOneTwo",
        2 => "ExecuteCircuitTwoTwo",
        3 => "ExecuteCircuitThreeTwo",
        4 => "ExecuteCircuitFourTwo",
        _ => panic!("There are only circuits for 1, 2, 3 or 4 input notes"),
    };
    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .expect(&format!(
            "There should be circuit data stored for {}",
            circuit_name
        ));
    let (pk, vd) = circuit_profile
        .get_keys()
        .expect(&format!("there should be keys stored for {}", circuit_name));

    let prover = Prover::try_from_bytes(pk).unwrap();
    let verifier = Verifier::try_from_bytes(vd).unwrap();

    (prover, verifier)
}

/// Executes a transaction.
/// Returns result containing gas spent.
pub fn execute(session: &mut Session, tx: Transaction) -> Result<u64, Error> {
    let mut receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        &tx,
        tx.payload().fee().gas_limit,
    )?;

    // Ensure all gas is consumed if there's an error in the contract call
    if receipt.data.is_err() {
        receipt.gas_spent = receipt.gas_limit;
    }

    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &(tx.payload().fee, receipt.gas_spent),
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    Ok(receipt.gas_spent)
}

/// Returns vector of notes owned by a given view key.
pub fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: ViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter()
        .filter(|note| vk.owns(note.stealth_address()))
        .collect()
}

/// Generate a TxCircuit given the sender secret-key, receiver public-key, the
/// input note positions in the transaction tree and the new output-notes.
pub fn create_transaction<const I: usize>(
    session: &mut Session,
    sender_sk: &SecretKey,
    receiver_pk: &PublicKey,
    gas_limit: u64,
    gas_price: u64,
    input_pos: [u64; I],
    transfer_value: u64,
    is_obfuscated: bool,
    deposit: u64,
    contract_call: Option<ContractCall>,
) -> Transaction {
    let mut rng = StdRng::seed_from_u64(0xfeeb);
    let sender_vk = ViewKey::from(sender_sk);
    let sender_pk = PublicKey::from(sender_sk);

    // Create the transaction payload:

    // Set the fee.
    let fee = Fee::new(&mut rng, &sender_pk, gas_limit, gas_price);
    let max_fee = fee.max_fee();

    // Get the root of the tree of phoenix-notes.
    let root = root(session).expect("Getting the anchor should be successful");

    // Get input notes, their openings and nullifier.
    let mut input_notes = Vec::with_capacity(I);
    let mut input_openings = Vec::with_capacity(I);
    let mut input_nullifiers = Vec::with_capacity(I);
    let mut input_value = 0;
    for pos in input_pos {
        // fetch the note and opening for the given position
        let leaves = leaves_from_pos(session, pos)
            .expect("Getting leaves in the given range should succeed");
        assert!(
            leaves.len() > 0,
            "There should be a note at the given position"
        );
        let note = &leaves[0].note;
        let opening = opening(session, pos)
            .expect(
                "Querying the opening for the given position should succeed",
            )
            .expect("An opening should exist for a note in the tree");
        // sanity check of the merkle opening
        assert!(opening.verify(poseidon_merkle::Item::new(
            rusk_abi::poseidon_hash(note.hash_inputs().to_vec()),
            ()
        )));

        input_notes.push(note.clone());
        input_openings.push(opening);
        input_nullifiers.push(note.gen_nullifier(&sender_sk));
        input_value += note
            .value(Some(&sender_vk))
            .expect("Note should be belonging to the sender");
    }

    // Generate output notes:
    assert!(input_value >= transfer_value + max_fee + deposit);
    let transfer_value_blinder = if is_obfuscated {
        JubJubScalar::random(&mut rng)
    } else {
        JubJubScalar::zero()
    };
    let transfer_sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    let change_sender_blinder = [
        JubJubScalar::random(&mut rng),
        JubJubScalar::random(&mut rng),
    ];
    let transfer_note = if is_obfuscated {
        Note::obfuscated(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            transfer_value,
            transfer_value_blinder,
            transfer_sender_blinder,
        )
    } else {
        Note::transparent(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            transfer_value,
            transfer_sender_blinder,
        )
    };
    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - transfer_value - max_fee - deposit;
    let change_value_blinder = JubJubScalar::random(&mut rng);
    let change_note = Note::obfuscated(
        &mut rng,
        &sender_pk,
        &sender_pk,
        change_value,
        change_value_blinder,
        change_sender_blinder,
    );
    let outputs = [transfer_note.clone(), change_note.clone()];

    let tx_skeleton = TxSkeleton {
        root,
        nullifiers: input_nullifiers,
        outputs,
        max_fee,
        deposit,
    };

    let tx_payload = Payload {
        tx_skeleton,
        fee,
        deposit: match deposit {
            0 => false,
            _ => true,
        },
        contract_call,
    };

    let payload_hash = tx_payload.hash();

    // Create the transaction proof

    // Create the `TxInputNote`
    let mut tx_input_notes = Vec::with_capacity(I);
    input_notes
        .iter()
        .zip(input_openings)
        .for_each(|(note, opening)| {
            tx_input_notes.push(
                TxInputNote::new(
                    &mut rng,
                    note,
                    opening,
                    &sender_sk,
                    payload_hash,
                )
                .expect("the sk should own the note"),
            );
        });

    // Create the `TxOutputNotes`
    let transfer_value_commitment =
        value_commitment(transfer_value, transfer_value_blinder);
    let transfer_note_sender_enc = match transfer_note.sender() {
        Sender::Encryption(enc) => enc,
        Sender::ContractInfo(_) => panic!("The sender is encrypted"),
    };
    let change_value_commitment =
        value_commitment(change_value, change_value_blinder);
    let change_note_sender_enc = match change_note.sender() {
        Sender::Encryption(enc) => enc,
        Sender::ContractInfo(_) => panic!("The sender is encrypted"),
    };
    let tx_output_notes = [
        TxOutputNote::new(
            transfer_value,
            transfer_value_commitment,
            transfer_value_blinder,
            JubJubAffine::from(
                transfer_note.stealth_address().note_pk().as_ref(),
            ),
            *transfer_note_sender_enc,
        ),
        TxOutputNote::new(
            change_value,
            change_value_commitment,
            change_value_blinder,
            JubJubAffine::from(
                change_note.stealth_address().note_pk().as_ref(),
            ),
            *change_note_sender_enc,
        ),
    ];

    // Sign the payload hash using both 'a' and 'b' of the sender_sk
    let schnorr_sk_a = SchnorrSecretKey::from(sender_sk.a());
    let sig_a = schnorr_sk_a.sign(&mut rng, payload_hash);
    let schnorr_sk_b = SchnorrSecretKey::from(sender_sk.b());
    let sig_b = schnorr_sk_b.sign(&mut rng, payload_hash);

    // Build the circuit
    let circuit: TxCircuit<TRANSFER_TREE_DEPTH, I> = TxCircuit::new(
        tx_input_notes
            .try_into()
            .expect("The input notes should be the correct ammount"),
        tx_output_notes,
        payload_hash,
        tx_payload.tx_skeleton().root,
        tx_payload.tx_skeleton().deposit,
        tx_payload.tx_skeleton().max_fee,
        sender_pk,
        (sig_a, sig_b),
        [transfer_sender_blinder, change_sender_blinder],
    );

    // fetch the prover and generate the proof
    let (prover, _verifier) = prover_verifier(input_pos.len());
    let (proof, _pi) = prover
        .prove(&mut rng, &circuit)
        .expect("creating a proof should succeed");

    // build the transaction from the payload and proof
    Transaction::new(tx_payload, proof.to_bytes().into())
}
