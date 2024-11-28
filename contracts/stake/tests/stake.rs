// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

use execution_core::{
    dusk,
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    stake::{
        Reward, RewardReason, Stake, StakeData, Withdraw as StakeWithdraw,
        STAKE_CONTRACT,
    },
    transfer::{
        data::ContractCall,
        phoenix::{
            PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
            ViewKey as PhoenixViewKey,
        },
        withdraw::{Withdraw, WithdrawReceiver, WithdrawReplayToken},
    },
    JubJubScalar, LUX,
};

use crate::common::assert::assert_event;
use crate::common::init::{instantiate, CHAIN_ID};
use crate::common::utils::*;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const POINT_LIMIT: u64 = 0x100_000_000;

#[test]
fn stake_withdraw_unstake() {
    // ------
    // instantiate the test
    const DEPOSIT_FEE: u64 = dusk(1.0);
    const WITHDRAW_FEE: u64 = dusk(1.0);
    const WITHDRAW_TRANSFER_FEE: u64 = dusk(1.0);
    const INITIAL_STAKE: u64 = GENESIS_VALUE / 2;

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

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

    let phoenix_receiver_pk = phoenix_sender_pk;
    let gas_limit = DEPOSIT_FEE;
    let gas_price = LUX;
    let transfer_value = 0;
    let is_obfuscated = false;
    let input_note_pos = 0;
    let deposit = INITIAL_STAKE;

    let chain_id =
        chain_id(&mut session).expect("Getting the chain ID should succeed");

    // Fashion a Stake struct
    let stake = Stake::new(&stake_sk, deposit, chain_id);
    let stake_bytes = rkyv::to_bytes::<_, 1024>(&stake)
        .expect("Should serialize Stake correctly")
        .to_vec();
    let contract_call = Some(ContractCall {
        contract: STAKE_CONTRACT,
        fn_name: String::from("stake"),
        fn_args: stake_bytes,
    });

    let tx = create_transaction(
        rng,
        &mut session,
        &phoenix_sender_sk,
        &phoenix_sender_pk,
        &phoenix_receiver_pk,
        gas_limit,
        gas_price,
        [input_note_pos],
        transfer_value,
        is_obfuscated,
        deposit,
        contract_call,
    );

    let receipt =
        execute(&mut session, tx).expect("Executing TX should succeed");

    assert_event(&receipt.events, "stake", &stake_pk, deposit);

    let gas_spent = receipt.gas_spent;
    receipt.data.expect("Executed TX should not error");
    update_root(&mut session).expect("Updating the root should succeed");

    println!("STAKE   : {gas_spent} gas");

    let stake_data: Option<StakeData> = session
        .call(STAKE_CONTRACT, "get_stake", &stake_pk, POINT_LIMIT)
        .expect("Getting the stake should succeed")
        .data;
    let stake_data =
        stake_data.expect("There should be a stake for the given key");

    let amount = stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount.value, deposit,
        "Staked amount should match sent amount"
    );
    assert_eq!(stake_data.reward, 0, "Initial reward should be zero");

    // ------
    // Add a reward to the staked key

    const REWARD_AMOUNT: u64 = dusk(5.0);

    let rewards = vec![Reward {
        account: stake_pk,
        value: REWARD_AMOUNT,
        reason: RewardReason::Other,
    }];

    let receipt = session
        .call::<_, ()>(STAKE_CONTRACT, "reward", &rewards, POINT_LIMIT)
        .expect("Rewarding a key should succeed");

    assert_event(&receipt.events, "reward", &stake_pk, REWARD_AMOUNT);

    let stake_data: Option<StakeData> = session
        .call(STAKE_CONTRACT, "get_stake", &stake_pk, POINT_LIMIT)
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
    let gas_limit = WITHDRAW_FEE;
    let gas_price = LUX;
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
        gas_limit,
        gas_price,
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
        .call(STAKE_CONTRACT, "get_stake", &stake_pk, POINT_LIMIT)
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
    let gas_limit = WITHDRAW_TRANSFER_FEE;
    let gas_price = LUX;
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
        gas_limit,
        gas_price,
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
