// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use execution_core::stake::{
    Reward, RewardReason, Stake, StakeData, Withdraw as StakeWithdraw, EPOCH,
    STAKE_CONTRACT,
};
use execution_core::transfer::data::ContractCall;
use execution_core::transfer::moonlight::Transaction as MoonlightTransaction;
use execution_core::transfer::withdraw::{
    Withdraw, WithdrawReceiver, WithdrawReplayToken,
};
use execution_core::transfer::TRANSFER_CONTRACT;
use execution_core::{dusk, ContractError, JubJubScalar, LUX};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;
// use rand::{CryptoRng, RngCore};
use rusk_abi::{CallReceipt, ContractData, Session, VM};

pub mod common;

use crate::common::assert::*;
use crate::common::init::CHAIN_ID;
use crate::common::utils::*;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const STAKE_VALUE: u64 = GENESIS_VALUE / 2;
const GENESIS_NONCE: u64 = 0;

#[test]
fn stake() {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");
    let mut session = instantiate(&mut vm, &moonlight_pk);

    // ------
    // Stake 1

    // execute 1st stake transaction
    let stake_1 = STAKE_VALUE / 3;
    let mut nonce = GENESIS_NONCE + 1;
    let receipt = execute_stake(
        &mut session,
        &moonlight_sk,
        nonce,
        &stake_sk,
        &stake_pk,
        stake_1,
        None,
    );
    assert_event(&receipt.events, "stake", &stake_pk, stake_1, 0);
    let gas_spent_1 = receipt.gas_spent;

    // verify 1st stake transaction
    assert_stake(&mut session, &stake_pk, stake_1, 0, 0);
    assert_moonlight(
        &mut session,
        &moonlight_pk,
        GENESIS_VALUE - stake_1 - gas_spent_1,
        nonce,
    );

    // ------
    // Stake 2

    // execute 2nd stake transaction
    let stake_2 = STAKE_VALUE / 3;
    nonce += 1;
    let receipt = execute_stake(
        &mut session,
        &moonlight_sk,
        nonce,
        &stake_sk,
        &stake_pk,
        stake_2,
        None,
    );
    assert_event(&receipt.events, "stake", &stake_pk, stake_2, 0);
    let gas_spent_2 = receipt.gas_spent;

    // verify 2nd stake transaction
    assert_stake(&mut session, &stake_pk, stake_1 + stake_2, 0, 0);
    assert_moonlight(
        &mut session,
        &moonlight_pk,
        GENESIS_VALUE - stake_1 - stake_2 - gas_spent_1 - gas_spent_2,
        nonce,
    );

    // ------
    // Lock

    // we need to manipulate the block-height so that the stake matures
    // commit the current session to a base commit
    let base = session.commit().expect("Committing should succeed");
    // start a new session from that base-commit with a new block-height
    let mut session = rusk_abi::new_session(&vm, base, CHAIN_ID, 2 * EPOCH)
        .expect("Instantiating new session should succeed");

    // execute 3rd stake transaction
    let stake_3 = STAKE_VALUE / 3;
    nonce += 1;
    let receipt = execute_stake(
        &mut session,
        &moonlight_sk,
        nonce,
        &stake_sk,
        &stake_pk,
        stake_3,
        None,
    );
    let expected_locked = stake_3 / 10;
    assert_event(
        &receipt.events,
        "stake",
        &stake_pk,
        stake_3 - expected_locked,
        expected_locked,
    );
    let gas_spent_3 = receipt.gas_spent;

    // verify 2nd stake transaction
    assert_stake(
        &mut session,
        &stake_pk,
        stake_1 + stake_2 + stake_3,
        expected_locked,
        0,
    );
    assert_moonlight(
        &mut session,
        &moonlight_pk,
        GENESIS_VALUE
            - stake_1
            - stake_2
            - stake_3
            - gas_spent_1
            - gas_spent_2
            - gas_spent_3,
        nonce,
    );
}

/*
        // ------
        // Add a reward to the staked key

        const REWARD_AMOUNT: u64 = dusk(5.0);

        let rewards = vec![Reward {
            account: stake_pk,
            value: REWARD_AMOUNT,
            reason: RewardReason::Other,
        }];

        let receipt = session
            .call::<_, ()>(STAKE_CONTRACT, "reward", &rewards, GAS_LIMIT)
            .expect("Rewarding a key should succeed");

        assert_event(&receipt.events, "reward", &stake_pk, REWARD_AMOUNT);

        let stake_data: Option<StakeData> = session
            .call(STAKE_CONTRACT, "get_stake", &stake_pk, GAS_LIMIT)
            .expect("Getting the stake should succeed")
            .data;
        let stake_data =
            stake_data.expect("There should be a stake for the given key");

        let amount = stake_data.amount.expect("There should be an amount staked");

        assert_eq!(
            amount.value, deposit,
            "Staked amount should match sent amount"
        );
        assert_eq!(
            stake_data.reward, REWARD_AMOUNT,
            "Reward should be set to specified amount"
        );

        // ------
        // Start withdrawing the reward just given to our key

        let leaves = leaves_from_height(&mut session, 1)
            .expect("Getting the notes should succeed");

        let input_notes = filter_notes_owned_by(
            phoenix_sender_vk,
            leaves.into_iter().map(|leaf| leaf.note),
        );

        assert_eq!(
            input_notes.len(),
            2,
            "All new notes should be owned by our view key"
        );

        let receiver_pk = phoenix_sender_pk;
        let input_positions = [*input_notes[0].pos(), *input_notes[1].pos()];
        let transfer_value = 0;
        let is_obfuscated = false;
        let deposit = 0;

        // Fashion a `Withdraw` struct instance
        let address =
            phoenix_sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
        let note_sk = phoenix_sender_sk.gen_note_sk(&address);

        let withdraw = Withdraw::new(
            rng,
            &note_sk,
            STAKE_CONTRACT,
            REWARD_AMOUNT,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Phoenix(vec![
                input_notes[0].gen_nullifier(&phoenix_sender_sk),
                input_notes[1].gen_nullifier(&phoenix_sender_sk),
            ]),
        );
        let withdraw = StakeWithdraw::new(&stake_sk, withdraw);

        let withdraw_bytes = rkyv::to_bytes::<_, 2048>(&withdraw)
            .expect("Serializing Withdraw should succeed")
            .to_vec();

        let contract_call = Some(ContractCall {
            contract: STAKE_CONTRACT,
            fn_name: String::from("withdraw"),
            fn_args: withdraw_bytes,
        });

        let tx = create_transaction(
            rng,
            &mut session,
            &phoenix_sender_sk,
            &phoenix_sender_pk,
            &receiver_pk,
            GAS_LIMIT,
            GAS_PRICE,
            input_positions,
            transfer_value,
            is_obfuscated,
            deposit,
            contract_call,
        );

        // set different block height so that the new notes are easily located and
        // filtered
        let base = session.commit().expect("Committing should succeed");
        let mut session = rusk_abi::new_session(vm, base, CHAIN_ID, 2)
            .expect("Instantiating new session should succeed");

        let receipt =
            execute(&mut session, tx).expect("Executing TX should succeed");

        assert_event(&receipt.events, "withdraw", &stake_pk, REWARD_AMOUNT);

        let gas_spent = receipt.gas_spent;
        receipt.data.expect("Executed TX should not error");
        update_root(&mut session).expect("Updating the root should succeed");

        println!("WITHDRAW: {gas_spent} gas");

        let stake_data: Option<StakeData> = session
            .call(STAKE_CONTRACT, "get_stake", &stake_pk, GAS_LIMIT)
            .expect("Getting the stake should succeed")
            .data;
        let stake_data =
            stake_data.expect("There should be a stake for the given key");

        let amount = stake_data.amount.expect("There should be an amount staked");

        assert_eq!(
            amount.value, INITIAL_STAKE,
            "Staked amount shouldn't have changed"
        );
        assert_eq!(stake_data.reward, 0, "Reward should be set to zero");

        // ------
        // Start unstaking the previously staked amount

        let leaves = leaves_from_height(&mut session, 2)
            .expect("Getting the notes should succeed");
        assert_eq!(
            leaves.len(),
            3,
            "There should be three notes in the tree at this block height \
            due to there there also a reward note having been produced"
        );

        let input_notes = filter_notes_owned_by(
            phoenix_sender_vk,
            leaves.into_iter().map(|leaf| leaf.note),
        );

        assert_eq!(
            input_notes.len(),
            3,
            "All new notes should be owned by our view key"
        );

        let receiver_pk = phoenix_sender_pk;
        let input_positions = [
            *input_notes[0].pos(),
            *input_notes[1].pos(),
            *input_notes[2].pos(),
        ];
        let transfer_value = 0;
        let is_obfuscated = false;
        let deposit = 0;

        // Fashion an `Unstake` struct instance
        let address =
            phoenix_sender_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
        let note_sk = phoenix_sender_sk.gen_note_sk(&address);

        let withdraw = Withdraw::new(
            rng,
            &note_sk,
            STAKE_CONTRACT,
            INITIAL_STAKE,
            WithdrawReceiver::Phoenix(address),
            WithdrawReplayToken::Phoenix(vec![
                input_notes[0].gen_nullifier(&phoenix_sender_sk),
                input_notes[1].gen_nullifier(&phoenix_sender_sk),
                input_notes[2].gen_nullifier(&phoenix_sender_sk),
            ]),
        );

        let unstake = StakeWithdraw::new(&stake_sk, withdraw);

        let unstake_bytes = rkyv::to_bytes::<_, 2048>(&unstake)
            .expect("Serializing Unstake should succeed")
            .to_vec();

        let contract_call = Some(ContractCall {
            contract: STAKE_CONTRACT,
            fn_name: String::from("unstake"),
            fn_args: unstake_bytes,
        });

        let tx = create_transaction(
            rng,
            &mut session,
            &phoenix_sender_sk,
            &phoenix_sender_pk,
            &receiver_pk,
            GAS_LIMIT,
            GAS_PRICE,
            input_positions,
            transfer_value,
            is_obfuscated,
            deposit,
            contract_call,
        );

        // set different block height so that the new notes are easily located and
        // filtered
        // sets the block height for all subsequent operations to 1
        let base = session.commit().expect("Committing should succeed");
        let mut session = rusk_abi::new_session(vm, base, CHAIN_ID, 3)
            .expect("Instantiating new session should succeed");

        let receipt =
            execute(&mut session, tx).expect("Executing TX should succeed");
        update_root(&mut session).expect("Updating the root should succeed");

        let gas_spent = receipt.gas_spent;
        receipt.data.expect("Executed TX should not error");

        println!("UNSTAKE : {gas_spent} gas");

        assert_event(&receipt.events, "unstake", &stake_pk, INITIAL_STAKE);
}
    */

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single moonlight account identified by the given public key, owning the
/// genesis-value.
fn instantiate(vm: &mut VM, moonlight_pk: &BlsPublicKey) -> Session {
    // create a new session using an ephemeral vm
    let mut session = rusk_abi::new_genesis_session(vm, CHAIN_ID);

    // deploy transfer-contract
    const OWNER: [u8; 32] = [0; 32];
    let transfer_bytecode = include_bytes!(
        "../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    session
        .deploy(
            transfer_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(TRANSFER_CONTRACT),
            GAS_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    // deploy stake-contract
    let stake_bytecode = include_bytes!(
        "../../../target/dusk/wasm32-unknown-unknown/release/stake_contract.wasm"
    );
    session
        .deploy(
            stake_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(STAKE_CONTRACT),
            GAS_LIMIT,
        )
        .expect("Deploying the stake contract should succeed");

    // insert genesis value to moonlight account
    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(*moonlight_pk, GENESIS_VALUE),
            GAS_LIMIT,
        )
        .expect("Inserting genesis account should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    let mut session = rusk_abi::new_session(vm, base, CHAIN_ID, 1)
        .expect("Instantiating new session should succeed");

    // check genesis state

    // the moonlight account is as expected
    assert_moonlight(&mut session, moonlight_pk, GENESIS_VALUE, GENESIS_NONCE);

    session
}

fn moonlight_contract_call(
    sender_sk: &BlsSecretKey,
    nonce: u64,
    deposit: u64,
    contract_call: ContractCall,
) -> MoonlightTransaction {
    let transfer_value = 0;
    MoonlightTransaction::new(
        sender_sk,
        None,
        transfer_value,
        deposit,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("Creating moonlight transaction should succeed")
}

fn execute_stake(
    session: &mut Session,
    moonlight_sk: &BlsSecretKey,
    nonce: u64,
    stake_sk: &BlsSecretKey,
    stake_pk: &BlsPublicKey,
    stake_value: u64,
    expected_error: Option<ContractError>,
) -> CallReceipt<Result<Vec<u8>, ContractError>> {
    // Fashion a Stake struct
    let stake = Stake::new(&stake_sk, stake_value, CHAIN_ID);
    let contract_call =
        ContractCall::new(STAKE_CONTRACT, String::from("stake"), &stake)
            .expect("Stake to serialize correctly");
    let tx = moonlight_contract_call(
        &moonlight_sk,
        nonce,
        stake_value,
        contract_call,
    );

    // execute moonlight tx
    let receipt = execute(session, tx).expect("Executing TX should succeed");

    let gas_spent = receipt.gas_spent;
    println!("STAKE   : {gas_spent} gas");

    match expected_error {
        Some(contract_error) => {
            assert_contract_error(&receipt.data, &contract_error);
        }
        None => {
            assert!(receipt.data.is_ok());
        }
    }

    receipt
}
