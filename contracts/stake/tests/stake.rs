// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_core::stake::{
    Reward, RewardReason, Stake, Withdraw as StakeWithdraw, STAKE_CONTRACT,
};
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::phoenix::{
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    ViewKey as PhoenixViewKey,
};
use dusk_core::transfer::withdraw::{
    Withdraw, WithdrawReceiver, WithdrawReplayToken,
};
use dusk_core::{dusk, JubJubScalar};
use dusk_vm::{execute, ExecutionConfig, VM};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub mod common;
use crate::common::assert::{
    assert_reward_event, assert_stake, assert_stake_event,
};
use crate::common::init::{instantiate, CHAIN_ID};
use crate::common::utils::*;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const INITIAL_STAKE: u64 = GENESIS_VALUE / 2;

const NO_CONFIG: ExecutionConfig = ExecutionConfig::DEFAULT;

#[test]
fn stake_withdraw_unstake() {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut VM::ephemeral().expect("Creating ephemeral VM should work");

    let phoenix_sender_sk = PhoenixSecretKey::random(rng);
    let phoenix_sender_vk = PhoenixViewKey::from(&phoenix_sender_sk);
    let phoenix_sender_pk = PhoenixPublicKey::from(&phoenix_sender_sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut session = instantiate(rng, vm, &phoenix_sender_pk, GENESIS_VALUE);

    let leaves = leaves_from_height(&mut session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    // ------
    // Stake

    // Fashion a Stake struct
    let stake = Stake::new(&stake_sk, &stake_sk, INITIAL_STAKE, CHAIN_ID);
    let stake_bytes = rkyv::to_bytes::<_, 1024>(&stake)
        .expect("Should serialize Stake correctly")
        .to_vec();
    let contract_call = Some(ContractCall {
        contract: STAKE_CONTRACT,
        fn_name: String::from("stake"),
        fn_args: stake_bytes,
    });

    let input_note_pos = 0;

    let tx = create_transaction(
        rng,
        &mut session,
        &phoenix_sender_sk,
        &phoenix_sender_pk,
        GAS_LIMIT,
        GAS_PRICE,
        [input_note_pos],
        INITIAL_STAKE,
        contract_call,
    );

    let receipt = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing TX should succeed");

    let gas_spent = receipt.gas_spent;
    receipt.data.expect("Executed TX should not error");
    update_root(&mut session).expect("Updating the root should succeed");

    println!("STAKE   : {gas_spent} gas");

    assert_stake_event(&receipt.events, "stake", &stake_pk, INITIAL_STAKE, 0);
    assert_stake(&mut session, &stake_pk, INITIAL_STAKE, 0, 0);

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

    assert_reward_event(&receipt.events, "reward", &stake_pk, REWARD_AMOUNT);
    assert_stake(&mut session, &stake_pk, INITIAL_STAKE, 0, REWARD_AMOUNT);

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

    let input_positions = [*input_notes[0].pos(), *input_notes[1].pos()];

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
    let withdraw = StakeWithdraw::new(&stake_sk, &stake_sk, withdraw);

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
        GAS_LIMIT,
        GAS_PRICE,
        input_positions,
        0,
        contract_call,
    );

    // set different block height so that the new notes are easily located and
    // filtered
    let base = session.commit().expect("Committing should succeed");
    let mut session = vm
        .session(base, CHAIN_ID, 2)
        .expect("Instantiating new session should succeed");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing TX should succeed");

    let gas_spent = receipt.gas_spent;
    receipt.data.expect("Executed TX should not error");
    update_root(&mut session).expect("Updating the root should succeed");

    println!("WITHDRAW: {gas_spent} gas");

    assert_stake_event(
        &receipt.events,
        "withdraw",
        &stake_pk,
        REWARD_AMOUNT,
        0,
    );
    assert_stake(&mut session, &stake_pk, INITIAL_STAKE, 0, 0);

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

    let input_positions = [
        *input_notes[0].pos(),
        *input_notes[1].pos(),
        *input_notes[2].pos(),
    ];

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

    let unstake = StakeWithdraw::new(&stake_sk, &stake_sk, withdraw);

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
        GAS_LIMIT,
        GAS_PRICE,
        input_positions,
        0,
        contract_call,
    );

    // set different block height so that the new notes are easily located and
    // filtered
    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");
    let mut session = vm
        .session(base, CHAIN_ID, 3)
        .expect("Instantiating new session should succeed");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)
        .expect("Executing TX should succeed");
    update_root(&mut session).expect("Updating the root should succeed");

    let gas_spent = receipt.gas_spent;
    receipt.data.expect("Executed TX should not error");

    println!("UNSTAKE : {gas_spent} gas");

    assert_stake_event(&receipt.events, "unstake", &stake_pk, INITIAL_STAKE, 0);
    assert_stake(&mut session, &stake_pk, 0, 0, 0);
}
