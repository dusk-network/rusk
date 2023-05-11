// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubScalar, GENERATOR_NUMS_EXTENDED};
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::transaction::*;
use phoenix_core::{Fee, Note};
use piecrust::Error;
use piecrust::{Session, VM};
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{ModuleError, RawResult};
use std::ops::Range;
use transfer_circuits::{
    CircuitInput, CircuitInputSignature, ExecuteCircuit, ExecuteCircuitOneTwo,
    ExecuteCircuitThreeTwo, ExecuteCircuitTwoTwo,
    SendToContractTransparentCircuit, WithdrawFromTransparentCircuit,
};

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const POINT_LIMIT: u64 = 0x10000000;

type Result<T, E = Error> = core::result::Result<T, E>;

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single note owned by the given public spend key.
fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &mut VM,
    psk: &PublicSpendKey,
    pk: &PublicKey,
) -> Session {
    rusk_abi::register_host_queries(vm);

    let transfer_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );
    let stake_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
    );

    let mut session = vm.genesis_session();

    session.set_point_limit(POINT_LIMIT);
    rusk_abi::set_block_height(&mut session, 0);

    session
        .deploy_with_id(rusk_abi::transfer_module(), transfer_bytecode)
        .expect("Deploying the transfer contract should succeed");

    session
        .deploy_with_id(rusk_abi::stake_module(), stake_bytecode)
        .expect("Deploying the stake contract should succeed");

    let genesis_note = Note::transparent(rng, psk, GENESIS_VALUE);

    // push genesis note to the contract
    let _: Note = session
        .transact(
            rusk_abi::transfer_module(),
            "push_note",
            &(0u64, genesis_note),
        )
        .expect("Pushing genesis note should succeed");

    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "update_root", &())
        .expect("Updating the root should succeed");

    let _: () = session
        .transact(rusk_abi::stake_module(), "add_owner", pk)
        .expect("Inserting APK into owners list should suceeed");

    // allow given public key to stake
    let _: () = session
        .transact(rusk_abi::stake_module(), "insert_allowlist", pk)
        .expect("Inserting APK into allowlist should succeed");

    // sets the block height for all subsequent operations to 1
    rusk_abi::set_block_height(&mut session, 1);

    session
}

fn leaves_in_range(
    session: &mut Session,
    range: Range<u64>,
) -> Result<Vec<TreeLeaf>> {
    session.query(
        rusk_abi::transfer_module(),
        "leaves_in_range",
        &(range.start, range.end),
    )
}

fn root(session: &mut Session) -> Result<BlsScalar> {
    session.query(rusk_abi::transfer_module(), "root", &())
}

fn opening(
    session: &mut Session,
    pos: u64,
) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>> {
    session.query(rusk_abi::transfer_module(), "opening", &pos)
}

fn prover_verifier<C: Circuit>(
    circuit_id: &[u8; 32],
) -> (Prover<C>, Verifier<C>) {
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

#[test]
fn stake_withdraw_unstake() {
    const STCT_FEE: u64 = dusk(1.0);
    const WITHDRAW_FEE: u64 = dusk(1.0);
    const WFCT_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut VM::ephemeral().expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    let vk = ssk.view_key();
    let psk = PublicSpendKey::from(&ssk);

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let session = &mut instantiate(rng, vm, &psk, &pk);

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
    // we transfer half of the input note to the stake contract, so the
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
    let stct_address = rusk_abi::module_to_scalar(&rusk_abi::stake_module());
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

    let stake_digest = stake_signature_message(0, crossover_value);
    let sig = sk.sign(&pk, &stake_digest);

    // Fashion a Stake struct
    let stake = Stake {
        public_key: pk,
        signature: sig,
        value: crossover_value,
        proof: stct_proof.to_bytes().to_vec(),
    };
    let stake_bytes = rkyv::to_bytes::<_, 4096>(&stake)
        .expect("Should serialize Stake correctly")
        .to_vec();

    let call = Some((
        rusk_abi::stake_module().to_bytes(),
        String::from("stake"),
        stake_bytes,
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

    let (prover_key, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &prover_key)
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

    session.set_point_limit(tx.fee.gas_limit * tx.fee.gas_price);
    let _: (u64, Option<Result<RawResult, ModuleError>>) = session
        .transact(rusk_abi::transfer_module(), "execute", &tx)
        .expect("Transacting should succeed");

    println!("STAKE   : {} gas", session.spent());

    let stake_data: Option<StakeData> = session
        .transact(rusk_abi::stake_module(), "get_stake", &pk)
        .expect("Getting the stake should succeed");
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount, crossover_value,
        "Staked amount should match sent amount"
    );
    assert_eq!(stake_data.reward, 0, "Initial reward should be zero");
    assert_eq!(stake_data.counter, 1, "Counter should increment once");

    // Add a reward to the staked key

    const REWARD_AMOUNT: u64 = dusk(5.0);

    let _: () = session
        .transact(rusk_abi::stake_module(), "reward", &(pk, REWARD_AMOUNT))
        .expect("Rewarding a key should succeed");

    let stake_data: Option<StakeData> = session
        .transact(rusk_abi::stake_module(), "get_stake", &pk)
        .expect("Getting the stake should succeed");
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount, crossover_value,
        "Staked amount should match sent amount"
    );
    assert_eq!(
        stake_data.reward, REWARD_AMOUNT,
        "Reward should be set to specified amount"
    );
    assert_eq!(stake_data.counter, 1, "Counter should increment once");

    // Start withdrawing the reward just given to our key

    let leaves = leaves_in_range(session, 1..2)
        .expect("Getting the notes should succeed");

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

    let gas_limit = WITHDRAW_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note = Note::obfuscated(rng, &psk, change_value, change_blinder);

    // Fashion a `Withdraw` struct instance

    let withdraw_address_r = JubJubScalar::random(rng);
    let withdraw_address = psk.gen_stealth_address(&withdraw_address_r);

    let withdraw_nonce = BlsScalar::random(rng);

    let withdraw_digest = withdraw_signature_message(
        stake_data.counter,
        withdraw_address,
        withdraw_nonce,
    );
    let withdraw_signature = sk.sign(&pk, &withdraw_digest);

    let withdraw = Withdraw {
        public_key: pk,
        signature: withdraw_signature,
        address: withdraw_address,
        nonce: withdraw_nonce,
    };
    let withdraw_bytes = rkyv::to_bytes::<_, 2048>(&withdraw)
        .expect("Serializing Withdraw should succeed")
        .to_vec();

    let call = Some((
        rusk_abi::stake_module().to_bytes(),
        String::from("withdraw"),
        withdraw_bytes,
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

    let (prover_key, _) =
        prover_verifier_keys(ExecuteCircuitTwoTwo::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &prover_key)
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

    // set different block height so that the new notes are easily located and
    // filtered
    rusk_abi::set_block_height(session, 2);

    session.set_point_limit(tx.fee.gas_limit * tx.fee.gas_price);
    let _: (u64, Option<Result<RawResult, ModuleError>>) = session
        .transact(rusk_abi::transfer_module(), "execute", &tx)
        .expect("Transacting should succeed");

    println!("WITHDRAW: {} gas", session.spent());

    let stake_data: Option<StakeData> = session
        .transact(rusk_abi::stake_module(), "get_stake", &pk)
        .expect("Getting the stake should succeed");
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount, crossover_value,
        "Staked amount should match sent amount"
    );
    assert_eq!(stake_data.reward, 0, "Reward should be set to zero");
    assert_eq!(stake_data.counter, 2, "Counter should increment once");

    // Start unstaking the previously staked amount

    let leaves = leaves_in_range(session, 2..3)
        .expect("Getting the notes should succeed");
    assert_eq!(
        leaves.len(),
        3,
        "There should be three notes in the tree at this block height \
        due to there there also a reward note having been produced"
    );

    let input_notes =
        filter_notes_owned_by(vk, leaves.into_iter().map(|leaf| leaf.note));

    assert_eq!(
        input_notes.len(),
        3,
        "All new notes should be owned by our view key"
    );

    let mut input_values = [0u64; 3];
    let mut input_blinders = [JubJubScalar::zero(); 3];
    let mut input_nullifiers = [BlsScalar::zero(); 3];

    for i in 0..3 {
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

    // Fashion a WFCT proof and an `Unstake` struct instance

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

    let unstake_digest =
        unstake_signature_message(stake_data.counter, withdraw_note);
    let unstake_sig = sk.sign(&pk, &unstake_digest);

    let unstake = Unstake {
        public_key: pk,
        signature: unstake_sig,
        note: withdraw_note,
        proof: wfct_proof.to_bytes().to_vec(),
    };
    let unstake_bytes = rkyv::to_bytes::<_, 2048>(&unstake)
        .expect("Serializing Unstake should succeed")
        .to_vec();

    let call = Some((
        rusk_abi::stake_module().to_bytes(),
        String::from("unstake"),
        unstake_bytes,
    ));

    // Compose the circuit. In this case we're using three inputs and one
    // output.
    let mut execute_circuit = ExecuteCircuit::new(3);

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
    let input_opening_2 = opening(session, *input_notes[2].pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r_0 = ssk.sk_r(input_notes[0].stealth_address());
    let pk_r_p_0 = GENERATOR_NUMS_EXTENDED * sk_r_0.as_ref();
    let sk_r_1 = ssk.sk_r(input_notes[1].stealth_address());
    let pk_r_p_1 = GENERATOR_NUMS_EXTENDED * sk_r_1.as_ref();
    let sk_r_2 = ssk.sk_r(input_notes[2].stealth_address());
    let pk_r_p_2 = GENERATOR_NUMS_EXTENDED * sk_r_2.as_ref();

    // The transaction hash must be computed before signing
    let anchor =
        root(session).expect("Getting the anchor should be successful");

    let tx_hash_input_bytes = Transaction::hash_input_bytes_from_components(
        &[
            input_nullifiers[0],
            input_nullifiers[1],
            input_nullifiers[2],
        ],
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
    let circuit_input_signature_2 =
        CircuitInputSignature::sign(rng, &ssk, &input_notes[2], tx_hash);

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
    let circuit_input_2 = CircuitInput::new(
        input_opening_2,
        input_notes[2],
        pk_r_p_2.into(),
        input_values[2],
        input_blinders[2],
        input_nullifiers[2],
        circuit_input_signature_2,
    );

    execute_circuit.add_input(circuit_input_0);
    execute_circuit.add_input(circuit_input_1);
    execute_circuit.add_input(circuit_input_2);

    let (prover_key, _) =
        prover_verifier_keys(ExecuteCircuitThreeTwo::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &prover_key)
        .expect("Proving should be successful");

    let tx = Transaction {
        anchor,
        nullifiers: vec![
            input_nullifiers[0],
            input_nullifiers[1],
            input_nullifiers[2],
        ],
        outputs: vec![change_note],
        fee,
        crossover: None,
        proof: execute_proof.to_bytes().to_vec(),
        call,
    };

    // set different block height so that the new notes are easily located and
    // filtered
    rusk_abi::set_block_height(session, 3);

    session.set_point_limit(tx.fee.gas_limit * tx.fee.gas_price);
    let _: (u64, Option<Result<RawResult, ModuleError>>) = session
        .transact(rusk_abi::transfer_module(), "execute", &tx)
        .expect("Transacting should succeed");

    println!("UNSTAKE : {} gas", session.spent());
}

#[test]
fn allow() {
    const ALLOW_FEE: u64 = dusk(1.0);

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut VM::ephemeral().expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    // let vk = ssk.view_key();
    let psk = PublicSpendKey::from(&ssk);

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let allow_sk = SecretKey::random(rng);
    let allow_pk = PublicKey::from(&allow_sk);

    let session = &mut instantiate(rng, vm, &psk, &pk);

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

    let gas_limit = ALLOW_FEE;
    let gas_price = LUX;

    // Since we're transferring value to a contract, a crossover is needed. Here
    // we transfer half of the input note to the stake contract, so the
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

    // Fashion a Allow struct
    let allow_digest = allow_signature_message(0, allow_pk);
    let allow_sig = sk.sign(&pk, &allow_digest);

    let allow = Allow {
        public_key: allow_pk,
        owner: pk,
        signature: allow_sig,
    };
    let allow_bytes = rkyv::to_bytes::<_, 2048>(&allow)
        .expect("Should serialize Allow correctly")
        .to_vec();

    let call = Some((
        rusk_abi::stake_module().to_bytes(),
        String::from("allow"),
        allow_bytes,
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

    let (prover_key, _) =
        prover_verifier_keys(ExecuteCircuitOneTwo::circuit_id());
    let (execute_proof, _) = execute_circuit
        .prove(rng, &prover_key)
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

    session.set_point_limit(tx.fee.gas_limit * tx.fee.gas_price);
    let _: (u64, Option<Result<RawResult, ModuleError>>) = session
        .transact(rusk_abi::transfer_module(), "execute", &tx)
        .expect("Transacting should succeed");

    println!("ALLOW   : {} gas", session.spent());

    let is_allowed: bool = session
        .query(rusk_abi::stake_module(), "is_allowlisted", &allow_pk)
        .expect("Querying the allowlist should succeed");

    assert!(is_allowed, "The new public key should now be allowed");
}
