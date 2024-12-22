// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_core::abi::Event;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    Reward, SlashEvent, StakeData, StakeEvent, STAKE_CONTRACT,
};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_vm::Session;
use rkyv::{check_archived_root, Deserialize, Infallible};

use super::utils::GAS_LIMIT;

pub fn assert_stake_event<S>(
    events: &Vec<Event>,
    topic: S,
    should_pk: &BlsPublicKey,
    should_value: u64,
    should_locked: u64,
) where
    S: AsRef<str>,
{
    let topic = topic.as_ref();
    let event = events
        .iter()
        .find(|e| e.topic == topic)
        .expect(&format!("event: {topic} should exist in the event list",));

    if topic == "stake" || topic == "unstake" || topic == "withdraw" {
        let staking_event_data =
            check_archived_root::<StakeEvent>(event.data.as_slice())
                .expect("Stake event data should deserialize correctly");
        let staking_event_data: StakeEvent = staking_event_data
            .deserialize(&mut Infallible)
            .expect("Infallible");
        assert_eq!(
            staking_event_data.value, should_value,
            "Stake-event: value incorrect"
        );
        assert_eq!(
            staking_event_data.locked, should_locked,
            "Stake-event: locked incorrect"
        );
        assert_eq!(
            staking_event_data.keys.account.to_bytes(),
            should_pk.to_bytes(),
            "Stake-event: stake key incorrect"
        );
    } else {
        panic!("{topic} topic cannot be verified with assert_stake_event");
    }
}

pub fn assert_reward_event<S>(
    events: &Vec<Event>,
    topic: S,
    should_pk: &BlsPublicKey,
    should_value: u64,
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
            &reward.account == should_pk && reward.value == should_value
        }))
    } else {
        panic!("{topic} topic cannot be verified with assert_reward_event");
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

pub fn assert_stake(
    session: &mut Session,
    stake_pk: &BlsPublicKey,
    expected_total: u64,
    expected_locked: u64,
    expected_reward: u64,
) {
    let stake_data: Option<StakeData> = session
        .call(STAKE_CONTRACT, "get_stake", stake_pk, GAS_LIMIT)
        .expect("Getting the stake should succeed")
        .data;

    if expected_total != 0 || expected_reward != 0 {
        let stake_data =
            stake_data.expect("There should be a stake for the given key");

        let amount =
            stake_data.amount.expect("There should be an amount staked");

        assert_eq!(
            amount.total_funds(),
            expected_total,
            "Total stake incorrect"
        );
        assert_eq!(amount.locked, expected_locked, "Locked stake incorrect");
        assert_eq!(
            stake_data.reward, expected_reward,
            "Stake reward incorrect"
        );
    } else {
        assert!(stake_data.is_none());
    }
}

pub fn assert_moonlight(
    session: &mut Session,
    moonlight_pk: &BlsPublicKey,
    expected_balance: u64,
    expected_nonce: u64,
) {
    let moonlight_account: AccountData = session
        .call(TRANSFER_CONTRACT, "account", moonlight_pk, GAS_LIMIT)
        .map(|r| r.data)
        .expect("Getting the moonlight account should succeed");
    assert_eq!(
        moonlight_account.balance, expected_balance,
        "Moonlight balance incorrect"
    );
    assert_eq!(
        moonlight_account.nonce, expected_nonce,
        "Moonlight nonce incorrect"
    );
}
