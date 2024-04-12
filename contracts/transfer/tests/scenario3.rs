// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use crate::common::utils::*;

use dusk_bls12_381_sign::{
    PublicKey as SignPublicKey, SecretKey as SignSecretKey,
};
use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubScalar, GENERATOR_NUMS_EXTENDED};
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey};
use phoenix_core::transaction::*;
use phoenix_core::{Fee, Note};
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{ContractData, ContractId, Session, TRANSFER_CONTRACT, VM};
use subsidy_types::Subsidy;
use transfer_circuits::{
    CircuitInput, CircuitInputSignature, ExecuteCircuitOneTwo,
    SendToContractTransparentCircuit,
};

const GENESIS_VALUE: u64 = dusk(1_000.0);
const POINT_LIMIT: u64 = 0x10000000;

const CHARLIE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFC;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [0; 32];

fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &VM,
    psk: Option<PublicSpendKey>,
    charlie_owner: Option<PublicSpendKey>,
) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let charlie_bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/charlie.wasm"
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

    if let Some(charlie_owner) = charlie_owner {
        session
            .deploy(
                charlie_bytecode,
                ContractData::builder()
                    .owner(charlie_owner.to_bytes())
                    .contract_id(CHARLIE_CONTRACT_ID),
                POINT_LIMIT,
            )
            .expect("Deploying the charlie contract should succeed");
    }

    if let Some(psk) = psk {
        let genesis_note = Note::transparent(rng, &psk, GENESIS_VALUE);

        // push genesis note to the contract
        session
            .call::<_, Note>(
                TRANSFER_CONTRACT,
                "push_note",
                &(0u64, genesis_note),
                POINT_LIMIT,
            )
            .expect("Pushing genesis note should succeed");
    }

    update_root(&mut session).expect("Updating the root should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    rusk_abi::new_session(vm, base, 1)
        .expect("Instantiating new session should succeed")
}

/// Transfers value from the given note into the contract's account.
/// Expects transparent note which will fund the subsidy and a value (amount)
/// which should be smaller or equal to the value of the note.
/// Returns the gas spent on the operation.
fn subsidize_contract<R: RngCore + CryptoRng>(
    rng: &mut R,
    mut session: &mut Session,
    contract_id: ContractId,
    subsidy_keeper_pk: SignPublicKey,
    subsidy_keeper_sk: SignSecretKey,
    subsidizer_psk: PublicSpendKey,
    subsidizer_ssk: SecretSpendKey,
    input_note: Note,
    crossover_value: u64,
) -> u64 {
    let input_note_value = input_note
        .value(None)
        .expect("The value should be transparent");
    let input_blinder = input_note
        .blinding_factor(None)
        .expect("The blinder should be transparent");
    let input_nullifier = input_note.gen_nullifier(&subsidizer_ssk);

    let gas_limit = dusk(1.0);
    let gas_price = LUX;

    assert!(crossover_value <= input_note_value);
    let crossover_blinder = JubJubScalar::random(rng);

    let (mut fee, crossover) = Note::obfuscated(
        rng,
        &subsidizer_psk,
        crossover_value,
        crossover_blinder,
    )
    .try_into()
    .expect("Getting a fee and a crossover should succeed");

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    let change_value =
        input_note_value - crossover_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    let change_note =
        Note::obfuscated(rng, &subsidizer_psk, change_value, change_blinder);

    let stct_address = rusk_abi::contract_to_scalar(&CHARLIE_CONTRACT_ID);
    let stct_signature = SendToContractTransparentCircuit::sign(
        rng,
        &subsidizer_ssk,
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

    let stake_digest = stake_signature_message(0, crossover_value);
    let sig = subsidy_keeper_sk.sign(&subsidy_keeper_pk, &stake_digest);

    let subsidy = Subsidy {
        public_key: subsidy_keeper_pk,
        signature: sig,
        value: crossover_value,
        proof: stct_proof.to_bytes().to_vec(),
    };
    let subsidy_bytes = rkyv::to_bytes::<_, 4096>(&subsidy)
        .expect("Should serialize Stake correctly")
        .to_vec();

    let call = Some((
        contract_id.to_bytes(),
        String::from("subsidize"),
        subsidy_bytes,
    ));

    let mut execute_circuit = ExecuteCircuitOneTwo::new();

    execute_circuit.set_fee_crossover(
        &fee,
        &crossover,
        crossover_value,
        crossover_blinder,
    );

    execute_circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending output should succeed");

    let input_opening = opening(&mut session, *input_note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    let sk_r = subsidizer_ssk.sk_r(input_note.stealth_address());
    let pk_r_p = GENERATOR_NUMS_EXTENDED * sk_r.as_ref();

    let anchor =
        root(&mut session).expect("Getting the anchor should be successful");

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
        CircuitInputSignature::sign(rng, &subsidizer_ssk, &input_note, tx_hash);
    let circuit_input = CircuitInput::new(
        input_opening,
        input_note,
        pk_r_p.into(),
        input_note_value,
        input_blinder,
        input_nullifier,
        circuit_input_signature,
    );

    execute_circuit
        .add_input(circuit_input)
        .expect("appending input should succeed");

    let (prover_key, _) = prover_verifier("ExecuteCircuitOneTwo");
    let (execute_proof, _) = prover_key
        .prove(rng, &execute_circuit)
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

    let gas_spent =
        execute(&mut session, tx).expect("Executing TX should succeed");
    update_root(&mut session).expect("Updating the root should succeed");
    gas_spent
}

fn instantiate_and_subsidize_contract(
    vm: &mut VM,
    contract_id: ContractId,
) -> (Session, SecretSpendKey) {
    const SUBSIDY_VALUE: u64 = GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let subsidizer_ssk = SecretSpendKey::random(rng); // money giver to subsidize the sponsor
    let subsidizer_psk = PublicSpendKey::from(&subsidizer_ssk);

    let test_sponsor_ssk = SecretSpendKey::random(rng);
    let test_sponsor_psk = PublicSpendKey::from(&test_sponsor_ssk); // sponsor is Charlie's owner

    let subsidy_keeper_sk = SignSecretKey::random(rng);
    let subsidy_keeper_pk = SignPublicKey::from(&subsidy_keeper_sk);

    let mut session =
        instantiate(rng, vm, Some(subsidizer_psk), Some(test_sponsor_psk));

    let leaves = leaves_from_height(&mut session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    let note = leaves[0].note;

    assert_eq!(
        module_balance(&mut session, contract_id)
            .expect("Module balance should succeed"),
        0u64
    );

    let _gas_spent = subsidize_contract(
        rng,
        &mut session,
        contract_id,
        subsidy_keeper_pk,
        subsidy_keeper_sk,
        subsidizer_psk,
        subsidizer_ssk,
        note,
        SUBSIDY_VALUE,
    );

    assert_eq!(
        module_balance(&mut session, contract_id)
            .expect("Module balance should succeed"),
        SUBSIDY_VALUE
    );

    println!("contract has been subsidized with amount={SUBSIDY_VALUE}");

    (session, test_sponsor_ssk)
}

fn call_contract_method(
    mut session: &mut Session,
    contract_id: ContractId,
    method: impl AsRef<str>,
    sponsor_ssk: SecretSpendKey,
) -> (u64, u64, u64) {
    const PING_FEE: u64 = dusk(1.0);
    const SPONSORING_NOTE_VALUE: u64 = 100_000_000_000;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);
    let test_sponsor_psk = PublicSpendKey::from(&sponsor_ssk); // sponsor is Charlie's owner

    // make sure the sponsoring contract is properly subsidized (has funds)
    let balance_before = module_balance(&mut session, contract_id)
        .expect("Module balance should succeed");
    println!(
        "current balance of contract '{:X?}' is {}",
        contract_id.to_bytes()[0],
        balance_before
    );

    let note = Note::transparent(rng, &test_sponsor_psk, SPONSORING_NOTE_VALUE);

    let note = session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, note),
            POINT_LIMIT,
        )
        .expect("Pushing genesis note should succeed")
        .data;

    update_root(&mut session).expect("Updating the root should succeed");

    let input_value =
        note.value(None).expect("The value should be transparent");
    println!(
        "sponsoring note has been obtained, note value={}",
        input_value
    );
    let input_blinder = note
        .blinding_factor(None)
        .expect("The blinder should be transparent");

    let input_nullifier = note.gen_nullifier(&sponsor_ssk);

    let gas_limit = PING_FEE;
    let gas_price = LUX;

    let fee = Fee::new(rng, gas_limit, gas_price, &test_sponsor_psk);

    // The change note should have the value of the input note, minus what is
    // maximally spent.
    let change_value = input_value - gas_price * gas_limit;
    let change_blinder = JubJubScalar::random(rng);
    println!("prepared change note with change value={}", change_value);
    let change_note =
        Note::obfuscated(rng, &test_sponsor_psk, change_value, change_blinder);

    let call = Some((
        contract_id.to_bytes(),
        String::from(method.as_ref()),
        vec![],
    ));

    // Compose the circuit. In this case we're using one input and one output.
    let mut circuit = ExecuteCircuitOneTwo::new();

    circuit.set_fee(&fee);
    circuit
        .add_output_with_data(change_note, change_value, change_blinder)
        .expect("appending input or output should succeed");

    let opening = opening(session, *note.pos())
        .expect("Querying the opening for the given position should succeed")
        .expect("An opening should exist for a note in the tree");

    // Generate pk_r_p
    let sk_r = sponsor_ssk.sk_r(note.stealth_address());
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
        CircuitInputSignature::sign(rng, &sponsor_ssk, &note, tx_hash);
    let circuit_input = CircuitInput::new(
        opening,
        note,
        pk_r_p.into(),
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

    println!(
        "executing method '{}' - contract '{:X?}' is paying",
        method.as_ref(),
        contract_id.to_bytes()[0]
    );
    let gas_spent = execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!(
        "gas spent for the execution of method '{}' is {}",
        method.as_ref(),
        gas_spent
    );

    let balance_after = module_balance(&mut session, contract_id)
        .expect("Module balance should succeed");

    println!(
        "contract's '{:X?}' balance before the call: {}",
        contract_id.as_bytes()[0],
        balance_before
    );
    println!(
        "contract's '{:X?}' balance after the call: {}",
        contract_id.as_bytes()[0],
        balance_after
    );
    if balance_before > balance_after {
        println!(
            "contract '{:X?}' has paid for this call: {}",
            contract_id.as_bytes()[0],
            balance_before - balance_after
        );
        println!("this call was sponsored by contract '{:X?}', gas spent by the caller is: {}", contract_id.as_bytes()[0], gas_spent);
    } else {
        println!(
            "contract '{:X?}' has earned: {}",
            contract_id.as_bytes()[0],
            balance_after - balance_before
        );
        println!("this call was charged by contract '{:X?}', gas spent by the caller is: {}", contract_id.as_bytes()[0], gas_spent);
    }

    (gas_spent, balance_before, balance_after)
}

#[test]
fn contract_sponsors_a_call() {
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_ssk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (gas_spent, balance_before, balance_after) = call_contract_method(
        &mut session,
        CHARLIE_CONTRACT_ID,
        "pay",
        sponsor_ssk,
    );
    assert!(balance_after < balance_before);
    assert_eq!(gas_spent, 0);
}

#[test]
fn contract_sponsors_not_enough_allowance() {
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_ssk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (gas_spent, balance_before, balance_after) = call_contract_method(
        &mut session,
        CHARLIE_CONTRACT_ID,
        "pay_and_fail",
        sponsor_ssk,
    );
    assert_eq!(balance_after, balance_before);
    assert!(gas_spent > 0);
}

#[test]
fn contract_earns_a_fee() {
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_ssk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (gas_spent, balance_before, balance_after) = call_contract_method(
        &mut session,
        CHARLIE_CONTRACT_ID,
        "earn",
        sponsor_ssk,
    );
    assert!(balance_after > balance_before);
    assert!(balance_after - balance_before <= gas_spent);
}

#[test]
fn contract_earns_not_enough_charge() {
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_ssk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (gas_spent, balance_before, balance_after) = call_contract_method(
        &mut session,
        CHARLIE_CONTRACT_ID,
        "earn_and_fail",
        sponsor_ssk,
    );
    assert_eq!(balance_before, balance_after);
    assert!(gas_spent > 0);
}
