// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

use execution_core::stake::{Stake, StakeData, Unstake, Withdraw};
use execution_core::{
    transfer::ContractCall, BlsScalar, JubJubScalar, PublicKey, SecretKey,
    StakePublicKey, StakeSecretKey, ViewKey,
};
use rusk_abi::dusk::{dusk, LUX};
use rusk_abi::STAKE_CONTRACT;

use crate::common::assert::assert_event;
use crate::common::init::instantiate;
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

    let sender_sk = SecretKey::random(rng);
    let sender_vk = ViewKey::from(&sender_sk);
    let sender_pk = PublicKey::from(&sender_sk);

    let stake_sk = StakeSecretKey::random(rng);
    let stake_pk = StakePublicKey::from(&stake_sk);

    let mut session = instantiate(rng, vm, &sender_pk, GENESIS_VALUE);

    let leaves = leaves_from_height(&mut session, 0)
        .expect("Getting leaves in the given range should succeed");

    assert_eq!(leaves.len(), 1, "There should be one note in the state");

    // ------
    // Stake

    let receiver_pk = sender_pk;
    let gas_limit = DEPOSIT_FEE;
    let gas_price = LUX;
    let transfer_value = 0;
    let is_obfuscated = false;
    let input_note_pos = 0;
    let deposit = INITIAL_STAKE;

    // Fashion a Stake struct
    let stake_digest = Stake::signature_message(0, deposit);
    let stake_sig = stake_sk.sign(&stake_pk, &stake_digest);
    let stake = Stake {
        public_key: stake_pk,
        signature: stake_sig,
        value: deposit,
    };
    let stake_bytes = rkyv::to_bytes::<_, 1024>(&stake)
        .expect("Should serialize Stake correctly")
        .to_vec();
    let contract_call = Some(ContractCall {
        contract: STAKE_CONTRACT.to_bytes(),
        fn_name: String::from("stake"),
        fn_args: stake_bytes,
    });

    let tx = create_transaction(
        &mut session,
        &sender_sk,
        &receiver_pk,
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
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(amount, deposit, "Staked amount should match sent amount");
    assert_eq!(stake_data.reward, 0, "Initial reward should be zero");
    assert_eq!(stake_data.counter, 1, "Counter should increment once");

    // ------
    // Add a reward to the staked key

    const REWARD_AMOUNT: u64 = dusk(5.0);

    let receipt = session
        .call::<_, ()>(
            STAKE_CONTRACT,
            "reward",
            &(stake_pk, REWARD_AMOUNT),
            POINT_LIMIT,
        )
        .expect("Rewarding a key should succeed");

    assert_event(&receipt.events, "reward", &stake_pk, REWARD_AMOUNT);

    let stake_data: Option<StakeData> = session
        .call(STAKE_CONTRACT, "get_stake", &stake_pk, POINT_LIMIT)
        .expect("Getting the stake should succeed")
        .data;
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(amount, deposit, "Staked amount should match sent amount");
    assert_eq!(
        stake_data.reward, REWARD_AMOUNT,
        "Reward should be set to specified amount"
    );
    assert_eq!(stake_data.counter, 1, "Counter should increment once");

    // ------
    // Start withdrawing the reward just given to our key

    let leaves = leaves_from_height(&mut session, 1)
        .expect("Getting the notes should succeed");

    let input_notes = filter_notes_owned_by(
        sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );

    assert_eq!(
        input_notes.len(),
        2,
        "All new notes should be owned by our view key"
    );

    let receiver_pk = sender_pk;
    let gas_limit = WITHDRAW_FEE;
    let gas_price = LUX;
    let input_positions = [*input_notes[0].pos(), *input_notes[1].pos()];
    let transfer_value = 0;
    let is_obfuscated = false;
    let deposit = 0;

    // Fashion a `Withdraw` struct instance
    let withdraw_address_r = JubJubScalar::random(&mut *rng);
    let withdraw_address = sender_pk.gen_stealth_address(&withdraw_address_r);
    let withdraw_nonce = BlsScalar::random(&mut *rng);
    let withdraw_digest = Withdraw::signature_message(
        stake_data.counter,
        withdraw_address,
        withdraw_nonce,
    );
    let withdraw_signature = stake_sk.sign(&stake_pk, &withdraw_digest);
    let withdraw = Withdraw {
        public_key: stake_pk,
        signature: withdraw_signature,
        address: withdraw_address,
        nonce: withdraw_nonce,
    };
    let withdraw_bytes = rkyv::to_bytes::<_, 2048>(&withdraw)
        .expect("Serializing Withdraw should succeed")
        .to_vec();

    let contract_call = Some(ContractCall {
        contract: STAKE_CONTRACT.to_bytes(),
        fn_name: String::from("withdraw"),
        fn_args: withdraw_bytes,
    });

    let tx = create_transaction(
        &mut session,
        &sender_sk,
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
    let mut session = rusk_abi::new_session(vm, base, 2)
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
    let stake_data = stake_data.expect("The stake should exist");

    let (amount, _) =
        stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount, INITIAL_STAKE,
        "Staked amount shouldn't have changed"
    );
    assert_eq!(stake_data.reward, 0, "Reward should be set to zero");
    assert_eq!(stake_data.counter, 2, "Counter should increment once");

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
        sender_vk,
        leaves.into_iter().map(|leaf| leaf.note),
    );

    assert_eq!(
        input_notes.len(),
        3,
        "All new notes should be owned by our view key"
    );

    let receiver_pk = sender_pk;
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
    let value = stake_data.amount.expect("There should be a stake").0;
    let address =
        receiver_pk.gen_stealth_address(&JubJubScalar::random(&mut *rng));
    let unstake_digest =
        Unstake::signature_message(stake_data.counter, value, address);
    let unstake_sig = stake_sk.sign(&stake_pk, unstake_digest.as_slice());

    let unstake = Unstake {
        public_key: stake_pk,
        signature: unstake_sig,
        address,
    };
    let unstake_bytes = rkyv::to_bytes::<_, 2048>(&unstake)
        .expect("Serializing Unstake should succeed")
        .to_vec();

    let contract_call = Some(ContractCall {
        contract: STAKE_CONTRACT.to_bytes(),
        fn_name: String::from("unstake"),
        fn_args: unstake_bytes,
    });

    let tx = create_transaction(
        &mut session,
        &sender_sk,
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
    let mut session = rusk_abi::new_session(vm, base, 3)
        .expect("Instantiating new session should succeed");

    let receipt =
        execute(&mut session, tx).expect("Executing TX should succeed");
    update_root(&mut session).expect("Updating the root should succeed");

    let gas_spent = receipt.gas_spent;
    receipt.data.expect("Executed TX should not error");

    println!("UNSTAKE : {gas_spent} gas");

    assert_event(&receipt.events, "unstake", &stake_pk, INITIAL_STAKE);
}
