// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use rkyv::{check_archived_root, Deserialize, Infallible};

use execution_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    stake::{Reward, SlashEvent, StakeEvent},
    Event,
};

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
