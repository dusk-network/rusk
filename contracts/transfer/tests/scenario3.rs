// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use crate::common::{
    contract_balance, create_transaction, execute, update_root, ExecutionResult,
};

use dusk_bytes::Serializable;
use execution_core::{
    stake::Stake, transfer::ContractCall, BlsPublicKey, BlsSecretKey,
    BlsSignature, JubJubScalar, Note, PublicKey, SecretKey,
};
use ff::Field;
use rand::rngs::StdRng;
use rand::{CryptoRng, RngCore, SeedableRng};
use rkyv::{Archive, Deserialize, Serialize};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::{
    ContractData, ContractId, EconomicMode, Session, TRANSFER_CONTRACT, VM,
};

const GENESIS_VALUE: u64 = dusk(1_000.0);
const POINT_LIMIT: u64 = 0x10_000_000;

const CHARLIE_CONTRACT_ID: ContractId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFC;
    ContractId::from_bytes(bytes)
};

const OWNER: [u8; 32] = [0; 32];

/// Subsidy a contract with a value.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Subsidy {
    /// Public key to which the subsidy will belong.
    pub public_key: BlsPublicKey,
    /// Signature belonging to the given public key.
    pub signature: BlsSignature,
    /// Value of the subsidy.
    pub value: u64,
}

fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &VM,
    pk: Option<PublicKey>,
    charlie_owner: Option<PublicKey>,
) -> Session {
    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    let charlie_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/charlie.wasm"
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

    if let Some(pk) = pk {
        let sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];
        let genesis_note =
            Note::transparent(rng, &pk, &pk, GENESIS_VALUE, sender_blinder);

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

/// Transfers value from given note into contract's account.
/// Expects transparent note which will fund the subsidy and a subsidy value
/// which is smaller or equal to the value of the note.
/// Returns the gas spent on the operation.
fn subsidize_contract(
    mut session: &mut Session,
    contract_id: ContractId,
    subsidy_keeper_pk: BlsPublicKey,
    subsidy_keeper_sk: BlsSecretKey,
    subsidizer_sk: &SecretKey,
    input_note_pos: u64,
    subsidy_value: u64,
) -> ExecutionResult {
    let receiver_pk = PublicKey::from(subsidizer_sk);
    let gas_limit = dusk(1.0);
    let gas_price = LUX;
    let input_pos = [input_note_pos];
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit = subsidy_value;

    let sig = subsidy_keeper_sk.sign(
        &subsidy_keeper_pk,
        &Stake::signature_message(0, subsidy_value),
    );

    let subsidy = Subsidy {
        public_key: subsidy_keeper_pk,
        signature: sig,
        value: subsidy_value,
    };
    let subsidy_bytes = rkyv::to_bytes::<_, 1024>(&subsidy)
        .expect("Subsidy should be correctly serialized")
        .to_vec();

    let contract_call = Some(ContractCall {
        contract: contract_id.to_bytes(),
        fn_name: String::from("subsidize"),
        fn_args: subsidy_bytes,
    });

    let tx = create_transaction(
        session,
        subsidizer_sk,
        &receiver_pk,
        gas_limit,
        gas_price,
        input_pos,
        transfer_value,
        is_obfuscated,
        deposit,
        contract_call,
    );

    let execution_result =
        execute(&mut session, tx).expect("Executing TX should succeed");
    update_root(&mut session).expect("Updating the root should succeed");
    execution_result
}

fn instantiate_and_subsidize_contract(
    vm: &mut VM,
    contract_id: ContractId,
) -> (Session, SecretKey) {
    const SUBSIDY_VALUE: u64 = GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let subsidizer_sk = SecretKey::random(rng); // money giver to subsidize the sponsor
    let subsidizer_pk = PublicKey::from(&subsidizer_sk);

    let charlie_owner_sk = SecretKey::random(rng);
    let charlie_owner_pk = PublicKey::from(&charlie_owner_sk); // sponsor is Charlie's owner

    let subsidy_keeper_sk = BlsSecretKey::random(rng);
    let subsidy_keeper_pk = BlsPublicKey::from(&subsidy_keeper_sk);

    let mut session =
        instantiate(rng, vm, Some(subsidizer_pk), Some(charlie_owner_pk));

    assert_eq!(
        contract_balance(&mut session, contract_id)
            .expect("Contract balance should succeed"),
        0u64
    );

    subsidize_contract(
        &mut session,
        contract_id,
        subsidy_keeper_pk,
        subsidy_keeper_sk,
        &subsidizer_sk,
        0,
        SUBSIDY_VALUE,
    );

    assert_eq!(
        contract_balance(&mut session, contract_id)
            .expect("Contract balance should succeed"),
        SUBSIDY_VALUE
    );

    println!("contract has been subsidized with amount={SUBSIDY_VALUE}");

    (session, charlie_owner_sk)
}

/// Creates and executes a transaction
/// which calls a given method of a given contract.
/// The transaction will contain input and output notes.
/// The contract is expected to have funds in its wallet.
fn call_contract_method_with_deposit(
    mut session: &mut Session,
    contract_id: ContractId,
    method: impl AsRef<str>,
    sponsor_sk: &SecretKey,
    gas_price: u64,
) -> (ExecutionResult, u64, u64) {
    const SPONSORING_NOTE_VALUE: u64 = 100_000_000_000;
    let rng = &mut StdRng::seed_from_u64(0xfeeb);
    let receiver_pk = PublicKey::from(sponsor_sk);
    let sender_pk = PublicKey::from(&SecretKey::random(rng));

    // make sure the sponsoring contract is properly subsidized (has funds)
    let balance_before = contract_balance(&mut session, contract_id)
        .expect("Contract balance should succeed");
    println!(
        "current balance of contract '{:X?}' is {}",
        contract_id.to_bytes()[0],
        balance_before
    );
    assert!(balance_before > 0);

    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let note = Note::transparent(
        rng,
        &sender_pk,
        &receiver_pk,
        SPONSORING_NOTE_VALUE,
        sender_blinder,
    );

    let note = session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, note),
            POINT_LIMIT,
        )
        .expect("Pushing new note should succeed")
        .data;
    update_root(&mut session).expect("Updating the root should succeed");

    let gas_limit = POINT_LIMIT;
    let input_pos = [*note.pos()];
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit = 0;

    let contract_call = Some(ContractCall {
        contract: contract_id.to_bytes(),
        fn_name: String::from(method.as_ref()),
        fn_args: vec![],
    });

    let tx = create_transaction(
        session,
        sponsor_sk,
        &receiver_pk,
        gas_limit,
        gas_price,
        input_pos,
        transfer_value,
        is_obfuscated,
        deposit,
        contract_call,
    );

    println!(
        "executing method '{}' - contract '{:X?}' is paying",
        method.as_ref(),
        contract_id.to_bytes()[0]
    );
    let execution_result =
        execute(session, tx).expect("Executing TX should succeed");
    update_root(session).expect("Updating the root should succeed");

    println!(
        "gas spent for the execution of method '{}' is {}",
        method.as_ref(),
        execution_result.gas_spent
    );

    let balance_after = contract_balance(&mut session, contract_id)
        .expect("Contract balance should succeed");

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

    (execution_result, balance_before, balance_after)
}

#[test]
fn contract_pays_for_call_with_deposit() {
    const GAS_PRICE: u64 = 2;
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_sk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (execution_result, balance_before, balance_after) =
        call_contract_method_with_deposit(
            &mut session,
            CHARLIE_CONTRACT_ID,
            "pay",
            &sponsor_sk,
            GAS_PRICE,
        );
    assert!(balance_after < balance_before);
    let balance_delta = balance_before - balance_after;
    if let EconomicMode::Allowance(allowance) = execution_result.economic_mode {
        assert!(allowance >= balance_delta)
    } else {
        assert!(false);
    }
    assert!(balance_delta >= execution_result.gas_spent);
}

#[test]
fn contract_pays_not_enough_allowance() {
    const GAS_PRICE: u64 = 2;
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_sk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (execution_result, balance_before, balance_after) =
        call_contract_method_with_deposit(
            &mut session,
            CHARLIE_CONTRACT_ID,
            "pay_and_fail",
            &sponsor_sk,
            GAS_PRICE,
        );
    assert_eq!(balance_after, balance_before);
    assert!(execution_result.gas_spent > 0);
}

#[test]
fn contract_does_not_pay_indirectly() {
    const GAS_PRICE: u64 = 2;
    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let (mut session, sponsor_sk) =
        instantiate_and_subsidize_contract(vm, CHARLIE_CONTRACT_ID);
    let (execution_result, balance_before, balance_after) =
        call_contract_method_with_deposit(
            &mut session,
            CHARLIE_CONTRACT_ID,
            "pay_indirectly_and_fail",
            &sponsor_sk,
            GAS_PRICE,
        );
    assert_eq!(balance_after, balance_before);
    assert_eq!(execution_result.economic_mode, EconomicMode::None);
}
