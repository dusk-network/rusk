// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubScalar, GENERATOR_NUMS_EXTENDED};
use dusk_pki::{Ownable, PublicKey, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use phoenix_core::transaction::*;
use phoenix_core::{Fee, Message, Note};
use piecrust::{ContractData, Error};
use piecrust::{Session, VM};
use poseidon_merkle::Opening as PoseidonOpening;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{ContractId, TRANSFER_CONTRACT};
use std::ops::Range;
use transfer_circuits::{
    CircuitInput, CircuitInputSignature, DeriveKey, ExecuteCircuit,
    ExecuteCircuitOneTwo, ExecuteCircuitTwoTwo,
    SendToContractObfuscatedCircuit, SendToContractTransparentCircuit,
    StcoCrossover, StcoMessage, WfoChange, WfoCommitment,
    WithdrawFromObfuscatedCircuit, WithdrawFromTransparentCircuit,
};

const GENESIS_VALUE: u64 = dusk(1_000.0);
const POINT_LIMIT: u64 = 0x10000000;
const GAS_PER_TX: u64 = 10_000;

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
    psk: &PublicSpendKey,
) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );
    let alice_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
    );
    let bob_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/alice.wasm"
    );

    let mut session = rusk_abi::new_genesis_session(vm);
    session.set_point_limit(POINT_LIMIT);

    session
        .deploy(
            transfer_bytecode,
            ContractData::builder(OWNER).contract_id(TRANSFER_CONTRACT),
        )
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy(
            alice_bytecode,
            ContractData::builder(OWNER).contract_id(ALICE_ID),
        )
        .expect("Deploying the alice contract should succeed");

    session
        .deploy(
            bob_bytecode,
            ContractData::builder(OWNER).contract_id(BOB_ID),
        )
        .expect("Deploying the bob contract should succeed");

    let genesis_note = Note::transparent(rng, psk, GENESIS_VALUE);

    // push genesis note to the contract
    let _: Note = session
        .call(TRANSFER_CONTRACT, "push_note", &(0u64, genesis_note))
        .expect("Pushing genesis note should succeed");

    update_root(&mut session).expect("Updating the root should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");
    let mut session = rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed");
    session.set_point_limit(POINT_LIMIT);

    session
}

fn leaves_in_range(
    session: &mut Session,
    range: Range<u64>,
) -> Result<Vec<TreeLeaf>> {
    session.call(
        TRANSFER_CONTRACT,
        "leaves_in_range",
        &(range.start, range.end),
    )
}

fn update_root(session: &mut Session) -> Result<()> {
    session.call(TRANSFER_CONTRACT, "update_root", &())
}

fn root(session: &mut Session) -> Result<BlsScalar> {
    session.call(TRANSFER_CONTRACT, "root", &())
}

fn module_balance(session: &mut Session, contract: ContractId) -> Result<u64> {
    session.call(TRANSFER_CONTRACT, "module_balance", &contract)
}

fn message(
    session: &mut Session,
    contract: ContractId,
    pk: PublicKey,
) -> Result<Option<Message>> {
    session.call(TRANSFER_CONTRACT, "message", &(contract, pk))
}

fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, 4>>> {
    session.call(TRANSFER_CONTRACT, "opening", &pos)
}

fn prover_verifier(circuit_id: &[u8; 32]) -> (Prover, Verifier) {
    let (pk, vd) = prover_verifier_keys(circuit_id);

    let prover = Prover::try_from_bytes(pk).unwrap();
    let verifier = Verifier::try_from_bytes(vd).unwrap();

    (prover, verifier)
}

fn prover_verifier_keys(circuit_id: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let keys = rusk_profile::keys_for(circuit_id).unwrap();

    let pk = keys.get_prover().unwrap();
    let vd = keys.get_verifier().unwrap();

    (pk, vd)
}

fn filter_notes_owned_by<I: IntoIterator<Item = Note>>(
    vk: ViewKey,
    iter: I,
) -> Vec<Note> {
    iter.into_iter().filter(|note| vk.owns(note)).collect()
}

/// Executes a transaction, returning the gas spent.
fn execute(session: &mut Session, tx: Transaction) -> Result<u64> {
    session.set_point_limit(u64::MAX);
    session.call(TRANSFER_CONTRACT, "spend", &tx)?;

    let mut gas_spent = GAS_PER_TX;
    if let Some((contract_id, fn_name, fn_data)) = &tx.call {
        let gas_limit = tx.fee.gas_limit - GAS_PER_TX;
        session.set_point_limit(gas_limit);

        let contract_id = ContractId::from_bytes(*contract_id);
        println!("Calling '{fn_name}' of {contract_id} with {gas_limit} gas");

        let r = session.call_raw(contract_id, fn_name, fn_data.clone());
        println!("{r:?}");

        gas_spent += session.spent();
    }

    session.set_point_limit(u64::MAX);
    let _: () = session
        .call(TRANSFER_CONTRACT, "refund", &(tx.fee, gas_spent))
        .expect("Refunding must succeed");

    Ok(gas_spent)
}

#[test]
fn transfer() {
    const TRANSFER_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    let psk = PublicSpendKey::from(&ssk);

    let receiver_ssk = SecretSpendKey::random(rng);
    let receiver_psk = PublicSpendKey::from(&receiver_ssk);

    let session = &mut instantiate(rng, vm, &psk);

    let leaves = leaves_in_range(session, 0..1)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&ssk);

    // Give half of the value of the note to the receiver.
    let output_value = input_value / 2;
    let output_blinder = JubJubScalar::random(rng);
    let output_note =
        Note::obfuscated(rng, &receiver_psk, output_value, output_blinder);

    let gas_limit = TRANSFER_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - output_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    // Compose the circuit. In this case we're using one input and two outputs.
    let mut circuit = ExecuteCircuit::new(1);

    circuit.set_fee(&fee);
    circuit.add_output_with_data(output_note, output_value, output_blinder);
    circuit.add_output_with_data(change_note, change_value, change_blinder);

    let opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r = ssk.sk_r(input_note.stealth_address());
    let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_note, tx_hash);
    let circuit_input = CircuitInput::<(), H, A>::new(
        opening,
        input_note,
        pk_r_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    circuit.add_input(circuit_input);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::<(), H, A>::circuit_id());
    let (proof, _) = circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

    let leaves = leaves_in_range(session, 1..2)
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

    let ssk = SecretSpendKey::random(rng);
    let psk = PublicSpendKey::from(&ssk);

    let session = &mut instantiate(rng, vm, &psk);

    let leaves = leaves_in_range(session, 0..1)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&ssk);

    let gas_limit = PING_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    let call = Some((ALICE_ID.to_bytes(), String::from("ping"), vec![]));

    // Compose the circuit. In this case we're using one input and one output.
    let mut circuit = ExecuteCircuit::new(1);

    circuit.set_fee(&fee);
    circuit.add_output_with_data(change_note, change_value, change_blinder);

    let opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r = ssk.sk_r(input_note.stealth_address());
    let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        opening,
        input_note,
        pk_r_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    circuit.add_input(circuit_input);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::<(), H, A>::circuit_id());
    let (proof, _) = circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

    let leaves = leaves_in_range(session, 1..2)
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

    let ssk = SecretSpendKey::random(rng);
    let vk = ssk.view_key();
    let psk = PublicSpendKey::from(&ssk);

    let session = &mut instantiate(rng, vm, &psk);

    let leaves = leaves_in_range(session, 0..1)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&ssk);

    let gas_limit = STCT_FEE;
    let gas_price = LUX;

    // Since we're transferring value to a contract, a crossover is needed. Here
    // we transfer half of the input note to the alice contract, so the
    // crossover value is `input_value/2`.
    let crossover_value = input_value / 2;
    let crossover_blinder = JubJubScalar::random(rng);

    let (mut fee, crossover) =
        Note::obfuscated(rng, &psk, crossover_value, crossover_blinder)
            .try_into()
            .expect("Getting a fee and a crossover should succeed");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - crossover_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    // Prove the STCT circuit.
    let stct_address = rusk_abi::contract_to_scalar(&ALICE_ID);
    let stct_signature = SendToContractTransparentCircuit::sign(
        rng,
        &ssk,
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

    let (prover, _) =
        prover_verifier(SendToContractTransparentCircuit::circuit_id());
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
    let mut execute_circuit = ExecuteCircuit::new(1);

    execute_circuit.set_fee_crossover(
        &fee,
        &crossover,
        crossover_value,
        crossover_blinder,
    );

    execute_circuit.add_output_with_data(
        change_note,
        change_value,
        change_blinder,
    );

    let input_opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r = ssk.sk_r(input_note.stealth_address());
    let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        input_opening,
        input_note,
        pk_r_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    execute_circuit.add_input(circuit_input);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::<(), H, A>::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

    let leaves = leaves_in_range(session, 1..2)
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
        input_nullifiers[i] = input_notes[i].gen_nullifier(&ssk);
    }

    let input_value: u64 = input_values.iter().sum();

    let gas_limit = WFCT_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    let withdraw_value = crossover_value;
    let withdraw_blinder = JubJubScalar::random(rng);
    let withdraw_note =
        Note::obfuscated(rng, &psk, withdraw_value, withdraw_blinder);

    // Fashion a WFCT proof and a `Wfct` structure instance

    let wfct_circuit = WithdrawFromTransparentCircuit::new(
        *withdraw_note.value_commitment(),
        withdraw_value,
        withdraw_blinder,
    );
    let (wfct_prover, _) =
        prover_verifier(WithdrawFromTransparentCircuit::circuit_id());

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
    let mut execute_circuit = ExecuteCircuit::new(2);

    execute_circuit.set_fee(&fee);

    execute_circuit.add_output_with_data(
        change_note,
        change_value,
        change_blinder,
    );

    let input_opening_0 = opening(session, *input_notes[0].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");
    let input_opening_1 = opening(session, *input_notes[1].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r_0 = ssk.sk_r(input_notes[0].stealth_address());
    let pk_r_p_0 = GENERATOR_NUMS_EXTENDED * sk_r_0.as_ref();
    let sk_r_1 = ssk.sk_r(input_notes[1].stealth_address());
    let pk_r_p_1 = GENERATOR_NUMS_EXTENDED * sk_r_1.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_notes[0], tx_hash);
    let circuit_input_signature_1 =
        CircuitInputSignature::sign(rng, &ssk, &input_notes[1], tx_hash);

    let circuit_input_0 = CircuitInput::new(
        input_opening_0,
        input_notes[0],
        pk_r_p_0.into(),
        input_values[0],
        input_blinders[0],
        input_nullifiers[0],
        circuit_input_signature_0,
    );
    let circuit_input_1 = CircuitInput::new(
        input_opening_1,
        input_notes[1],
        pk_r_p_1.into(),
        input_values[1],
        input_blinders[1],
        input_nullifiers[1],
        circuit_input_signature_1,
    );

    execute_circuit.add_input(circuit_input_0);
    execute_circuit.add_input(circuit_input_1);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitTwoTwo::<(), H, A>::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

#[test]
fn send_and_withdraw_obfuscated() {
    const STCO_FEE: u64 = dusk(1.0);
    const WFCO_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    let vk = ssk.view_key();
    let psk = PublicSpendKey::from(&ssk);

    let session = &mut instantiate(rng, vm, &psk);

    let leaves = leaves_in_range(session, 0..1)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let input_note = leaves[0].note;
    let input_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&ssk);

    let gas_limit = STCO_FEE;
    let gas_price = LUX;

    // Since we're transferring value to a contract, a crossover is needed. Here
    // we transfer half of the input note to the alice contract, so the
    // crossover value is `input_value/2`.
    let crossover_value = input_value / 2;
    let crossover_blinder = JubJubScalar::random(rng);

    let (mut fee, crossover) =
        Note::obfuscated(rng, &psk, crossover_value, crossover_blinder)
            .try_into()
            .expect("Getting a fee and a crossover should succeed");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - crossover_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    // Prove the STCO circuit.

    let stco_address = rusk_abi::contract_to_scalar(&ALICE_ID);

    let stco_m_r = JubJubScalar::random(rng);
    let stco_m = Message::new(rng, &stco_m_r, &psk, crossover_value);

    let stco_signature = SendToContractObfuscatedCircuit::sign(
        rng,
        &ssk,
        &fee,
        &crossover,
        &stco_m,
        &stco_address,
    );

    let stco_m_address = psk.gen_stealth_address(&stco_m_r);
    let stco_m_address_pk_r = *stco_m_address.pk_r().as_ref();
    let (_, stco_m_blinder) = stco_m
        .decrypt(&stco_m_r, &psk)
        .expect("Should decrypt message successfully");

    let stco_derive_key = DeriveKey::new(false, &psk);

    let stco_message = StcoMessage {
        r: stco_m_r,
        blinder: stco_m_blinder,
        derive_key: stco_derive_key,
        pk_r: stco_m_address_pk_r,
        message: stco_m,
    };

    let stco_crossover = StcoCrossover::new(crossover, crossover_blinder);

    let stco_circuit = SendToContractObfuscatedCircuit::new(
        crossover_value,
        stco_message,
        stco_crossover,
        &fee,
        stco_address,
        stco_signature,
    );

    let (stco_prover, _) =
        prover_verifier(SendToContractObfuscatedCircuit::circuit_id());
    let (stco_proof, _) = stco_prover
        .prove(rng, &stco_circuit)
        .expect("Proving STCO circuit should succeed");

    // Fashion the STCO struct
    let stco = Stco {
        module: ALICE_ID.to_bytes(),
        message: stco_m,
        message_address: stco_m_address,
        proof: stco_proof.to_bytes().to_vec(),
    };
    let stco_bytes = rkyv::to_bytes::<_, 2048>(&stco)
        .expect("Should serialize Stco correctly")
        .to_vec();

    let call = Some((
        TRANSFER_CONTRACT.to_bytes(),
        String::from("stco"),
        stco_bytes,
    ));

    // Compose the circuit. In this case we're using one input and one output.
    let mut execute_circuit = ExecuteCircuit::new(1);

    execute_circuit.set_fee_crossover(
        &fee,
        &crossover,
        crossover_value,
        crossover_blinder,
    );

    execute_circuit.add_output_with_data(
        change_note,
        change_value,
        change_blinder,
    );

    let input_opening = opening(session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r = ssk.sk_r(input_note.stealth_address());
    let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        input_opening,
        input_note,
        pk_r_p.into(),
        input_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    execute_circuit.add_input(circuit_input);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::<(), H, A>::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

    println!("EXECUTE_STCO: {gas_spent} gas");

    let leaves = leaves_in_range(session, 1..2)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        2,
        "There should be two notes in the tree at this block height"
    );

    // the alice module has a message that contains the given value

    // Notice `stco_m_address` and `stco_m_r` need to be kept to retrieve the
    // message and decrypt the value and blinder. They are also used as inputs
    // to withdrawal.
    let wfco_input_message = message(session, ALICE_ID, *stco_m_address.pk_r())
        .expect("Querying for a message should succeed")
        .expect("The message should be present in the state");
    let (wfco_input_value, wfco_input_blinder) = wfco_input_message
        .decrypt(&stco_m_r, &psk)
        .expect("Decrypting the message should succeed");

    assert_eq!(
        wfco_input_value, crossover_value,
        "Message owned by Alice should have the correct value"
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
        input_nullifiers[i] = input_notes[i].gen_nullifier(&ssk);
    }

    let input_value: u64 = input_values.iter().sum();

    let gas_limit = WFCO_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    // Fashion a WFCO proof and a `Wfco` structure instance

    // We'll withdraw half of the deposited value
    let wfco_output_value = crossover_value / 2;

    assert_eq!(wfco_input_value, crossover_value, "Output should");

    let wfco_input = WfoCommitment {
        value: wfco_input_value,
        blinder: wfco_input_blinder,
        commitment: *wfco_input_message.value_commitment(),
    };

    let wfco_change_value = wfco_input_value - wfco_output_value;
    let wfco_change_r = JubJubScalar::random(rng);
    let wfco_change_address = psk.gen_stealth_address(&wfco_change_r);
    let wfco_change_message =
        Message::new(rng, &wfco_change_r, &psk, wfco_change_value);
    let (_, wfco_change_blinder) = wfco_change_message
        .decrypt(&wfco_change_r, &psk)
        .expect("Decrypting message should succeed");
    let wfco_change_pk_r = *wfco_change_address.pk_r().as_ref();
    let wfco_change_derive_key = DeriveKey::new(false, &psk);

    let wfco_change = WfoChange {
        value: wfco_change_value,
        message: wfco_change_message,
        blinder: wfco_change_blinder,
        r: wfco_change_r,
        derive_key: wfco_change_derive_key,
        pk_r: wfco_change_pk_r,
    };

    let wfco_output_blinder = JubJubScalar::random(rng);
    let wfco_output_note =
        Note::obfuscated(rng, &psk, wfco_output_value, wfco_output_blinder);
    let wfco_output_commitment = *wfco_output_note.value_commitment();

    let wfco_output = WfoCommitment {
        value: wfco_output_value,
        blinder: wfco_output_blinder,
        commitment: wfco_output_commitment,
    };

    let wfco_circuit = WithdrawFromObfuscatedCircuit {
        input: wfco_input,
        change: wfco_change,
        output: wfco_output,
    };
    let (wfco_prover, _) =
        prover_verifier(WithdrawFromObfuscatedCircuit::circuit_id());

    let (wfco_proof, _) = wfco_prover
        .prove(rng, &wfco_circuit)
        .expect("Proving WFCT circuit should succeed");

    let wfco = Wfco {
        message: wfco_input_message,
        message_address: stco_m_address,
        change: wfco_change_message,
        change_address: wfco_change_address,
        output: wfco_output_note,
        proof: wfco_proof.to_bytes().to_vec(),
    };
    let wfco_bytes = rkyv::to_bytes::<_, 2048>(&wfco)
        .expect("Serializing Wfct should succeed")
        .to_vec();

    let call = Some((
        ALICE_ID.to_bytes(),
        String::from("withdraw_obfuscated"),
        wfco_bytes,
    ));

    // Compose the circuit. In this case we're using two inputs and one output.
    let mut execute_circuit = ExecuteCircuit::new(2);

    execute_circuit.set_fee(&fee);

    execute_circuit.add_output_with_data(
        change_note,
        change_value,
        change_blinder,
    );

    let input_opening_0 = opening(session, *input_notes[0].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");
    let input_opening_1 = opening(session, *input_notes[1].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r_0 = ssk.sk_r(input_notes[0].stealth_address());
    let pk_r_p_0 = GENERATOR_NUMS_EXTENDED * sk_r_0.as_ref();
    let sk_r_1 = ssk.sk_r(input_notes[1].stealth_address());
    let pk_r_p_1 = GENERATOR_NUMS_EXTENDED * sk_r_1.as_ref();

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
        CircuitInputSignature::sign(rng, &ssk, &input_notes[0], tx_hash);
    let circuit_input_signature_1 =
        CircuitInputSignature::sign(rng, &ssk, &input_notes[1], tx_hash);

    let circuit_input_0 = CircuitInput::new(
        input_opening_0,
        input_notes[0],
        pk_r_p_0.into(),
        input_values[0],
        input_blinders[0],
        input_nullifiers[0],
        circuit_input_signature_0,
    );
    let circuit_input_1 = CircuitInput::new(
        input_opening_1,
        input_notes[1],
        pk_r_p_1.into(),
        input_values[1],
        input_blinders[1],
        input_nullifiers[1],
        circuit_input_signature_1,
    );

    execute_circuit.add_input(circuit_input_0);
    execute_circuit.add_input(circuit_input_1);

    let (pk, _) =
        prover_verifier_keys(ExecuteCircuitTwoTwo::<(), H, A>::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &pk)
        .expect("Proving should be successful");

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

    println!("EXECUTE_WFCO: {gas_spent} gas");

    // deposited message shouldn't exist after, and only the newly created one
    // should exists with the correct value

    assert!(
        message(session, ALICE_ID, *stco_m_address.pk_r())
            .expect("Querying for a message should succeed")
            .is_none(),
        "The previous message should not present in the state"
    );

    let wfco_change_message =
        message(session, ALICE_ID, *wfco_change_address.pk_r())
            .expect("Querying for a message should succeed")
            .expect("The message should be present in the state");
    let (wfco_change_value, _) = wfco_change_message
        .decrypt(&wfco_change_r, &psk)
        .expect("Decrypting the message should succeed");

    assert_eq!(
        wfco_change_value,
        crossover_value - wfco_output_value,
        "Remaining value should what was put in minus what is taken out"
    );
}
