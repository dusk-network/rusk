// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use execution_core::signatures::bls::PublicKey as BlsPublicKey;
use execution_core::stake::{
    Reward, SlashEvent, StakeData, StakeEvent, STAKE_CONTRACT,
};
use execution_core::transfer::moonlight::AccountData;
use execution_core::transfer::TRANSFER_CONTRACT;
use execution_core::{ContractError, Event};
use rkyv::{check_archived_root, Deserialize, Infallible};
use rusk_abi::Session;

use super::utils::GAS_LIMIT;

pub fn assert_event<S>(
    events: &Vec<Event>,
    topic: S,
    should_pk: &BlsPublicKey,
    should_amount: u64,
) where
    S: AsRef<str>,
{
    let topic = topic.as_ref();
    let event = events
        .iter()
        .find(|e| e.topic == topic)
        .expect(&format!("event: {topic} should exist in the event list",));

    if topic == "reward" {
        let reward_event_data = rkyv::from_bytes::<Vec<Reward>>(&event.data)
            .expect("Reward event data should deserialize correctly");

        assert!(reward_event_data.iter().any(|reward| {
            &reward.account == should_pk && reward.value == should_amount
        }))
    } else {
        let staking_event_data =
            check_archived_root::<StakeEvent>(event.data.as_slice())
                .expect("Stake event data should deserialize correctly");
        let staking_event_data: StakeEvent = staking_event_data
            .deserialize(&mut Infallible)
            .expect("Infallible");
        assert_eq!(staking_event_data.value, should_amount);
        assert_eq!(
            staking_event_data.keys.account.to_bytes(),
            should_pk.to_bytes()
        );
    }
}

pub fn assert_slash_event<S, E: Into<Option<u64>>>(
    events: &Vec<Event>,
    topic: S,
    should_pk: &BlsPublicKey,
    should_amount: u64,
    should_eligibility: E,
) where
    S: AsRef<str>,
{
    let topic = topic.as_ref();
    let event = events
        .iter()
        .find(|e| e.topic == topic)
        .expect(&format!("event: {topic} should exist in the event list",));

    if topic == "slash" || topic == "hard_slash" {
        let staking_event_data =
            check_archived_root::<SlashEvent>(event.data.as_slice())
                .expect("Stake event data should deserialize correctly");
        let staking_event_data: SlashEvent = staking_event_data
            .deserialize(&mut Infallible)
            .expect("Infallible");
        assert_eq!(staking_event_data.value, should_amount);
        assert_eq!(staking_event_data.account.to_bytes(), should_pk.to_bytes());
        let should_eligibility: Option<u64> = should_eligibility.into();
        if let Some(should_eligibility) = should_eligibility {
            assert_eq!(staking_event_data.next_eligibility, should_eligibility);
        }
    } else {
        panic!("{topic} topic cannot be verified with assert_slash_event");
    }
}

pub fn assert_moonlight(
    moonlight_pk: &BlsPublicKey,
    expected_balance: u64,
    expected_nonce: u64,
    session: &mut Session,
) {
    let moonlight_account: AccountData = session
        .call(TRANSFER_CONTRACT, "account", moonlight_pk, GAS_LIMIT)
        .map(|r| r.data)
        .expect("Getting the sender account should succeed");
    assert_eq!(
        moonlight_account.balance, expected_balance,
        "The sender moonlight account should have its genesis value"
    );
    assert_eq!(moonlight_account.nonce, expected_nonce);
}

pub fn assert_stake(
    stake_pk: &BlsPublicKey,
    expected_stake: u64,
    expected_reward: u64,
    session: &mut Session,
) {
    let stake_data: Option<StakeData> = session
        .call(STAKE_CONTRACT, "get_stake", stake_pk, GAS_LIMIT)
        .expect("Getting the stake should succeed")
        .data;
    let stake_data =
        stake_data.expect("There should be a stake for the given key");

    let amount = stake_data.amount.expect("There should be an amount staked");

    assert_eq!(
        amount.value, expected_stake,
        "Staked amount should match sent amount"
    );
    assert_eq!(
        stake_data.reward, expected_reward,
        "Initial reward should be zero"
    );
}

// ContractError doesn't impl PartialEq so adding this here
pub fn assert_contract_error(
    receipt_error: &Result<Vec<u8>, ContractError>,
    expected_error: &ContractError,
) {
    match (receipt_error, expected_error) {
        (
            Err(ContractError::Panic(receipt_msg)),
            ContractError::Panic(expected_msg),
        ) => assert!(receipt_msg == expected_msg),
        (Err(ContractError::OutOfGas), ContractError::OutOfGas) => {}
        (Err(ContractError::DoesNotExist), ContractError::DoesNotExist) => {}
        (Err(ContractError::Unknown), ContractError::Unknown) => {}
        _ => panic!(
            "contract error not as expected. Result: {:?}\nExpected: {:?}",
            receipt_error, expected_error
        ),
    }
}
