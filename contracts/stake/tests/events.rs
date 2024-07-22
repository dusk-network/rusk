// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use rand::rngs::StdRng;
use rand::SeedableRng;

use execution_core::{
    stake::{StakeAmount, StakeData},
    BlsPublicKey, BlsSecretKey, PublicKey, SecretKey,
};
use rusk_abi::dusk::dusk;
use rusk_abi::{PiecrustError, STAKE_CONTRACT, TRANSFER_CONTRACT};

use crate::common::assert::assert_event;
use crate::common::init::instantiate;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);

#[test]
fn reward_slash() -> Result<(), PiecrustError> {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut session = instantiate(rng, vm, &pk, GENESIS_VALUE);

    let reward_amount = dusk(10.0);
    let stake_amount = dusk(100.0);
    let slash_amount = dusk(5.0);

    let stake_data = StakeData {
        reward: 0,
        amount: Some(StakeAmount {
            value: stake_amount,
            eligibility: 0,
        }),
        nonce: 0,
        faults: 0,
        hard_faults: 0,
    };

    session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "add_contract_balance",
        &(STAKE_CONTRACT, stake_amount),
        u64::MAX,
    )?;

    session.call::<_, ()>(
        STAKE_CONTRACT,
        "insert_stake",
        &(stake_pk, stake_data),
        u64::MAX,
    )?;

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(stake_pk, reward_amount),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "reward", &stake_pk, reward_amount);

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "slash",
        &(stake_pk, Some(slash_amount)),
        u64::MAX,
    )?;
    assert!(receipt.events.len() == 1, "No shift at first warn");
    assert_event(&receipt.events, "slash", &stake_pk, slash_amount);
    let stake_amount = stake_amount - slash_amount;

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "slash",
        &(stake_pk, None::<u64>),
        u64::MAX,
    )?;
    // 10% of current amount
    let slash_amount = stake_amount / 10;
    assert_event(&receipt.events, "slash", &stake_pk, slash_amount);
    assert_event(&receipt.events, "suspended", &stake_pk, 4320);

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "slash",
        &(stake_pk, None::<u64>),
        u64::MAX,
    )?;
    let stake_amount = stake_amount - slash_amount;

    // 20% of current amount
    let slash_amount = stake_amount / 100 * 20;
    assert_event(&receipt.events, "slash", &stake_pk, slash_amount);
    assert_event(&receipt.events, "suspended", &stake_pk, 6480);

    Ok(())
}

#[test]
fn stake_hard_slash() -> Result<(), PiecrustError> {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut session = instantiate(rng, vm, &pk, GENESIS_VALUE);

    let stake_amount = dusk(100.0);
    let hard_slash_amount = dusk(5.0);
    let severity = 2;
    let reward_amount = dusk(10.0);
    let block_height = 0;

    let stake_data = StakeData {
        reward: 0,
        amount: Some(StakeAmount {
            value: stake_amount,
            eligibility: block_height,
        }),
        nonce: 0,
        faults: 0,
        hard_faults: 0,
    };

    session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "add_contract_balance",
        &(STAKE_CONTRACT, stake_amount),
        u64::MAX,
    )?;

    session.call::<_, ()>(
        STAKE_CONTRACT,
        "insert_stake",
        &(stake_pk, stake_data),
        u64::MAX,
    )?;

    let mut cur_balance = stake_amount;
    // Simple hard fault (slash 10%)
    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "hard_slash",
        &(stake_pk, None::<u64>, None::<u8>),
        u64::MAX,
    )?;
    let expected_slash = stake_amount / 100 * 10;
    assert_event(&receipt.events, "hard_slash", &stake_pk, expected_slash);
    cur_balance -= expected_slash;

    // Severe hard fault (slash 30%)
    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "hard_slash",
        &(stake_pk, None::<u64>, Some(severity as u8)),
        u64::MAX,
    )?;
    let expected_slash = cur_balance / 100 * (1 + severity) * 10;
    assert_event(&receipt.events, "hard_slash", &stake_pk, expected_slash);
    cur_balance -= expected_slash;

    // Direct slash (slash hard_slash_amount)
    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "hard_slash",
        &(stake_pk, Some(hard_slash_amount), None::<u8>),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "hard_slash", &stake_pk, hard_slash_amount);
    cur_balance -= hard_slash_amount;

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(stake_pk, reward_amount),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "reward", &stake_pk, reward_amount);

    // Simple hard fault post-reward (slash 10%)
    // Rewards should reset 'hard_faults'
    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "hard_slash",
        &(stake_pk, None::<u64>, None::<u8>),
        u64::MAX,
    )?;
    let expected_slash = cur_balance / 100 * 10;
    assert_event(&receipt.events, "hard_slash", &stake_pk, expected_slash);

    Ok(())
}
