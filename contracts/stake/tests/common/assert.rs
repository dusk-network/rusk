// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use rkyv::{check_archived_root, Deserialize, Infallible};
use rusk_abi::Event;
use stake_contract_types::StakingEvent;

pub fn assert_event<S>(
    events: &Vec<Event>,
    topic: S,
    should_pk: &PublicKey,
    should_amount: u64,
) where
    S: AsRef<str>,
{
    let event = events
        .iter()
        .find(|e| e.topic == topic.as_ref())
        .expect("event should exist in the event list");
    let staking_event_data =
        check_archived_root::<StakingEvent>(event.data.as_slice())
            .expect("Stake event data should deserialize correctly");
    let staking_event_data: StakingEvent = staking_event_data
        .deserialize(&mut Infallible)
        .expect("Infallible");
    assert_eq!(staking_event_data.value, should_amount);
    assert_eq!(
        staking_event_data.public_key.to_bytes(),
        should_pk.to_bytes()
    );
}
