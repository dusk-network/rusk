// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubScalar, GENERATOR_NUMS_EXTENDED};
use dusk_plonk::prelude::*;
use ff::Field;
use phoenix_core::transaction::*;
use phoenix_core::{Fee, Note, Ownable, PublicKey, SecretKey, ViewKey};
use poseidon_merkle::Opening as PoseidonOpening;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{
    ContractData, ContractError, ContractId, Error, Session, TRANSFER_CONTRACT,
    VM,
};
use transfer_circuits::{
    CircuitInput, CircuitInputSignature, ExecuteCircuitOneTwo,
    ExecuteCircuitTwoTwo, SendToContractTransparentCircuit,
    WithdrawFromTransparentCircuit,
};

const GENESIS_VALUE: u64 = dusk(1_000.0);
const POINT_LIMIT: u64 = 0x10000000;

const ALICE_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFA;
    ContractId::from_bytes(bytes)
};
const BOB_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFB;
    ContractId::from_bytes(bytes)
};

type Result<T, E = Error> = core::result::Result<T, E>;

const OWNER: [u8; 32] = [0; 32];

const H: usize = TRANSFER_TREE_DEPTH;
const A: usize = 4;

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single note owned by the given public spend key.
fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &VM,
    pk: &PublicKey,
) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let alice_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
    );
    let bob_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);

    session
        .deploy(
            transfer_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(TRANSFER_CONTRACT),
            POINT_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy(
            alice_bytecode,
            ContractData::builder().owner(OWNER).contract_id(ALICE_ID),
            POINT_LIMIT,
        )
        .expect("Deploying the alice contract should succeed");

    session
        .deploy(
            bob_bytecode,
            ContractData::builder().owner(OWNER).contract_id(BOB_ID),
            POINT_LIMIT,
        )
        .expect("Deploying the bob contract should succeed");

    let genesis_note = Note::transparent(rng, pk, GENESIS_VALUE);

    // push genesis note to the contract
    session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, genesis_note),
            POINT_LIMIT,
        )
        .expect("Pushing genesis note should succeed");

    update_root(&mut session).expect("Updating the root should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed")
}

fn leaves_from_height(
    session: &mut Session,
    height: u64,
) -> Result<Vec<TreeLeaf>> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_height",
        &height,
        u64::MAX,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

fn leaves_from_pos(session: &mut Session, pos: u64) -> Result<Vec<TreeLeaf>> {
    let (feeder, receiver) = mpsc::channel();

    session.feeder_call::<_, ()>(
        TRANSFER_CONTRACT,
        "leaves_from_pos",
        &pos,
        u64::MAX,
        feeder,
    )?;

    Ok(receiver
        .iter()
        .map(|bytes| rkyv::from_bytes(&bytes).expect("Should return leaves"))
        .collect())
}

fn num_notes(session: &mut Session) -> Result<u64> {
    session
        .call(TRANSFER_CONTRACT, "num_notes", &(), u64::MAX)
        .map(|r| r.data)
}

fn update_root(session: &mut Session) -> Result<()> {
    session
        .call(TRANSFER_CONTRACT, "update_root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

fn root(session: &mut Session) -> Result<BlsScalar> {
    session
        .call(TRANSFER_CONTRACT, "root", &(), POINT_LIMIT)
        .map(|r| r.data)
}

fn module_balance(session: &mut Session, contract: ContractId) -> Result<u64> {
    session
        .call(TRANSFER_CONTRACT, "module_balance", &contract, POINT_LIMIT)
        .map(|r| r.data)
}

fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, 4>>> {
    session
        .call(TRANSFER_CONTRACT, "opening", &pos, POINT_LIMIT)
        .map(|r| r.data)
}

fn prover_verifier(circuit_name: &str) -> (Prover, Verifier) {
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

fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: ViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter().filter(|note| vk.owns(note)).collect()
}

/// Executes a transaction, returning the gas spent.
fn execute(session: &mut Session, tx: Transaction) -> Result<u64> {
    let receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        &tx,
        u64::MAX,
    )?;

    let gas_spent = receipt.gas_spent;

    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &(tx.fee, gas_spent),
            u64::MAX,
        )
        .expect("Refunding must succeed");

    Ok(gas_spent)
}

#[test]
fn transfer() {
    const TRANSFER_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let receiver_sk = SecretKey::random(rng);
    let receiver_pk = PublicKey::from(&receiver_sk);

    let session = &mut instantiate(rng, vm, &pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let pos = num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        pos,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    let leaves = leaves_from_pos(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&sk);

    // Give half of the value of the note to the receiver.
    let output_value = input_value / 2;
    let output_blinder = JubJubScalar::random(&mut *rng);
    let output_note =
        Note::obfuscated(rng, &receiver_pk, output_value, output_blinder);

    let gas_limit = TRANSFER_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &pk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - output_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(&mut *rng);
    let change_note = Note::obfuscated(rng, &pk, change_value, change_blinder);

    // Compose the circuit. In this case we're using one input and two outputs.
    let mut circuit = ExecuteCircuitOneTwo::new();

    circuit.set_fee(&fee);
    circuit
        .add_output_with_data(output_note, output_value, output_blinder)
        .expect("appending input or output should succeed");
    circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending input or output should succeed");

    let opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate npk_p
    let nsk = sk.sk_r(input_note.stealth_address());
    let npk_p = GENERATOR_NUMS_EXTENDED * nsk.as_ref();

    // The transaction hash must be computed before signing
    let anchor =
        root(session).expect("Getting the anchor should be successful");

    let tx_hash_input_bytes = Transaction::hash_input_bytes_from_components(
        &[input_nullifier],
        &[output_note, change_note],
        &anchor,
        &fee,
        &None,
        &None,
    );
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    circuit.set_tx_hash(tx_hash);

    let circuit_input_signature =
        CircuitInputSignature::sign(rng, &sk, &input_note, tx_hash);
    let circuit_input = CircuitInput::<(), H, A>::new(
        opening,
        input_note,
        npk_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    circuit
        .add_input(circuit_input)
        .expect("appending input or output should succeed");

    let (prover, _) = prover_verifier("ExecuteCircuitOneTwo");
    let (proof, _) = prover
        .prove(rng, &circuit)
        .expect("creating a proof should succeed");

    let tx = Transaction {
        anchor,
        nullifiers: vec![input_nullifier],
        outputs: vec![output_note, change_note],
        fee,
        crossover: None,
        proof: proof.to_bytes().to_vec(),
        call: None,
    };

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_1_2 : {gas_spent} gas");

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "There should be three notes in the tree at this block height"
    );

    let pos = num_notes(session).expect("Getting num_notes should succeed");
    assert_eq!(
        pos,
        leaves.last().expect("note to exists").note.pos() + 1,
        "num_notes should match position of last note + 1"
    );

    let leaves = leaves_from_pos(session, input_note.pos() + 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "There should be three notes in the tree at this block height"
    );
}

#[test]
fn alice_ping() {
    const PING_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let session = &mut instantiate(rng, vm, &pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&sk);

    let gas_limit = PING_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &pk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(&mut *rng);
    let change_note = Note::obfuscated(rng, &pk, change_value, change_blinder);

    let call = Some((ALICE_ID.to_bytes(), String::from("ping"), vec![]));

    // Compose the circuit. In this case we're using one input and one output.
    let mut circuit = ExecuteCircuitOneTwo::new();

    circuit.set_fee(&fee);
    circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending input or output should succeed");

    let opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate npk_p
    let nsk = sk.sk_r(input_note.stealth_address());
    let npk_p = GENERATOR_NUMS_EXTENDED * nsk.as_ref();

    // The transaction hash must be computed before signing
    let anchor =
        root(session).expect("Getting the anchor should be successful");

    let tx_hash_input_bytes = Transaction::hash_input_bytes_from_components(
        &[input_nullifier],
        &[change_note],
        &anchor,
        &fee,
        &None,
        &call,
    );
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    circuit.set_tx_hash(tx_hash);

    let circuit_input_signature =
        CircuitInputSignature::sign(rng, &sk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        opening,
        input_note,
        npk_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    circuit
        .add_input(circuit_input)
        .expect("appending input or output should succeed");

    let (prover, _) = prover_verifier("ExecuteCircuitOneTwo");
    let (proof, _) = prover
        .prove(rng, &circuit)
        .expect("creating a proof should succeed");

    let tx = Transaction {
        anchor,
        nullifiers: vec![input_nullifier],
        outputs: vec![change_note],
        fee,
        crossover: None,
        proof: proof.to_bytes().to_vec(),
        call,
    };

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_PING: {gas_spent} gas");

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the tree after the transaction"
    );
}

#[test]
fn send_and_withdraw_transparent() {
    const STCT_FEE: u64 = dusk(1.0);
    const WFCT_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sk = SecretKey::random(rng);
    let vk = ViewKey::from(&sk);
    let pk = PublicKey::from(&sk);

    let session = &mut instantiate(rng, vm, &pk);

    let leaves = leaves_from_height(session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&sk);

    let gas_limit = STCT_FEE;
    let gas_price = LUX;

    // Since we're transferring value to a contract, a crossover is needed. Here
    // we transfer half of the input note to the alice contract, so the
    // crossover value is `input_value/2`.
    let crossover_value = input_value / 2;
    let crossover_blinder = JubJubScalar::random(&mut *rng);

    let (mut fee, crossover) =
        Note::obfuscated(rng, &pk, crossover_value, crossover_blinder)
            .try_into()
            .expect("Getting a fee and a crossover should succeed");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - crossover_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(&mut *rng);
    let change_note = Note::obfuscated(rng, &pk, change_value, change_blinder);

    // Prove the STCT circuit.
    let stct_address = rusk_abi::contract_to_scalar(&ALICE_ID);
    let stct_signature = SendToContractTransparentCircuit::sign(
        rng,
        &sk,
        &fee,
        &crossover,
        crossover_value,
        &stct_address,
    );

    let stct_circuit = SendToContractTransparentCircuit::new(
        &fee,
        &crossover,
        crossover_value,
        crossover_blinder,
        stct_address,
        stct_signature,
    );

    let (prover, _) = prover_verifier("SendToContractTransparentCircuit");
    let (stct_proof, _) = prover
        .prove(rng, &stct_circuit)
        .expect("Proving STCT circuit should succeed");

    // Fashion the STCT struct
    let stct = Stct {
        module: ALICE_ID.to_bytes(),
        value: crossover_value,
        proof: stct_proof.to_bytes().to_vec(),
    };
    let stct_bytes = rkyv::to_bytes::<_, 2048>(&stct)
        .expect("Should serialize Stct correctly")
        .to_vec();

    let call = Some((
        TRANSFER_CONTRACT.to_bytes(),
        String::from("stct"),
        stct_bytes,
    ));

    // Compose the circuit. In this case we're using one input and one output.
    let mut execute_circuit = ExecuteCircuitOneTwo::new();

    execute_circuit.set_fee_crossover(
        &fee,
        &crossover,
        crossover_value,
        crossover_blinder,
    );

    execute_circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending input or output should succeed");

    let input_opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate npk_p
    let nsk = sk.sk_r(input_note.stealth_address());
    let npk_p = GENERATOR_NUMS_EXTENDED * nsk.as_ref();

    // The transaction hash must be computed before signing
    let anchor =
        root(session).expect("Getting the anchor should be successful");

    let tx_hash_input_bytes = Transaction::hash_input_bytes_from_components(
        &[input_nullifier],
        &[change_note],
        &anchor,
        &fee,
        &Some(crossover),
        &call,
    );
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    execute_circuit.set_tx_hash(tx_hash);

    let circuit_input_signature =
        CircuitInputSignature::sign(rng, &sk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        input_opening,
        input_note,
        npk_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    execute_circuit
        .add_input(circuit_input)
        .expect("appending input or output should succeed");

    let (prover, _) = prover_verifier("ExecuteCircuitOneTwo");
    let (execute_proof, _) = prover
        .prove(rng, &execute_circuit)
        .expect("creating a proof should succeed");

    let tx = Transaction {
        anchor,
        nullifiers: vec![input_nullifier],
        outputs: vec![change_note],
        fee,
        crossover: Some(crossover),
        proof: execute_proof.to_bytes().to_vec(),
        call,
    };

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_STCT: {gas_spent} gas");

    let leaves = leaves_from_height(session, 1)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the tree at this block height"
    );

    // the alice module has the correct balance

    let alice_balance = module_balance(session, ALICE_ID)
        .expect("Querying the module balance should succeed");
    assert_eq!(
        alice_balance, crossover_value,
        "Alice should have the value of the input crossover"
    );

    // start withdrawing the amount just transferred to the alice contract
    // this is done by calling the alice contract directly, which then calls the
    // transfer module

    let input_notes =
        filter_notes_owned_by(vk, leaves.into_iter().map(|leaf| leaf.note));

    assert_eq!(
        input_notes.len(),
        2,
        "All new notes should be owned by our view key"
    );

    let mut input_values = [0u64; 2];
    let mut input_blinders = [JubJubScalar::zero(); 2];
    let mut input_nullifiers = [BlsScalar::zero(); 2];

    for i in 0..2 {
        input_values[i] = input_notes[i]
            .value(Some(&vk))
            .expect("The given view key should own the note");
        input_blinders[i] = input_notes[i]
            .blinding_factor(Some(&vk))
            .expect("The given view key should own the note");
        input_nullifiers[i] = input_notes[i].gen_nullifier(&sk);
    }

    let input_value: u64 = input_values.iter().sum();

    let gas_limit = WFCT_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &pk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(&mut *rng);
    let change_note = Note::obfuscated(rng, &pk, change_value, change_blinder);

    let withdraw_value = crossover_value;
    let withdraw_blinder = JubJubScalar::random(&mut *rng);
    let withdraw_note =
        Note::obfuscated(rng, &pk, withdraw_value, withdraw_blinder);

    // Fashion a WFCT proof and a `Wfct` structure instance

    let wfct_circuit = WithdrawFromTransparentCircuit::new(
        *withdraw_note.value_commitment(),
        withdraw_value,
        withdraw_blinder,
    );
    let (wfct_prover, _) = prover_verifier("WithdrawFromTransparentCircuit");

    let (wfct_proof, _) = wfct_prover
        .prove(rng, &wfct_circuit)
        .expect("Proving WFCT circuit should succeed");

    let wfct = Wfct {
        value: crossover_value,
        note: withdraw_note,
        proof: wfct_proof.to_bytes().to_vec(),
    };
    let wfct_bytes = rkyv::to_bytes::<_, 2048>(&wfct)
        .expect("Serializing Wfct should succeed")
        .to_vec();

    let call =
        Some((ALICE_ID.to_bytes(), String::from("withdraw"), wfct_bytes));

    // Compose the circuit. In this case we're using two inputs and one output.
    let mut execute_circuit = ExecuteCircuitTwoTwo::new();

    execute_circuit.set_fee(&fee);

    execute_circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending input or output should succeed");

    let input_opening_0 = opening(session, *input_notes[0].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");
    let input_opening_1 = opening(session, *input_notes[1].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate npk_p
    let nsk_0 = sk.sk_r(input_notes[0].stealth_address());
    let npk_p_0 = GENERATOR_NUMS_EXTENDED * nsk_0.as_ref();
    let nsk_1 = sk.sk_r(input_notes[1].stealth_address());
    let npk_p_1 = GENERATOR_NUMS_EXTENDED * nsk_1.as_ref();

    // The transaction hash must be computed before signing
    let anchor =
        root(session).expect("Getting the anchor should be successful");

    let tx_hash_input_bytes = Transaction::hash_input_bytes_from_components(
        &[input_nullifiers[0], input_nullifiers[1]],
        &[change_note],
        &anchor,
        &fee,
        &None,
        &call,
    );
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    execute_circuit.set_tx_hash(tx_hash);

    let circuit_input_signature_0 =
        CircuitInputSignature::sign(rng, &sk, &input_notes[0], tx_hash);
    let circuit_input_signature_1 =
        CircuitInputSignature::sign(rng, &sk, &input_notes[1], tx_hash);

    let circuit_input_0 = CircuitInput::new(
        input_opening_0,
        input_notes[0],
        npk_p_0.into(),
        input_values[0],
        input_blinders[0],
        input_nullifiers[0],
        circuit_input_signature_0,
    );
    let circuit_input_1 = CircuitInput::new(
        input_opening_1,
        input_notes[1],
        npk_p_1.into(),
        input_values[1],
        input_blinders[1],
        input_nullifiers[1],
        circuit_input_signature_1,
    );

    execute_circuit
        .add_input(circuit_input_0)
        .expect("appending input or output should succeed");
    execute_circuit
        .add_input(circuit_input_1)
        .expect("appending input or output should succeed");

    let (prover, _) = prover_verifier("ExecuteCircuitTwoTwo");
    let (execute_proof, _) = prover
        .prove(rng, &execute_circuit)
        .expect("creating a proof should succeed");

    let tx = Transaction {
        anchor,
        nullifiers: vec![input_nullifiers[0], input_nullifiers[1]],
        outputs: vec![change_note],
        fee,
        crossover: None,
        proof: execute_proof.to_bytes().to_vec(),
        call,
    };

    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!("EXECUTE_WFCT: {gas_spent} gas");

    let alice_balance = module_balance(session, ALICE_ID)
        .expect("Querying the module balance should succeed");
    assert_eq!(
        alice_balance, 0,
        "Alice should have no balance after it is withdrawn"
    );
}
