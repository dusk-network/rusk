// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_core::stake::{Reward, RewardReason, EPOCH, STAKE_CONTRACT};
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_vm::{
    new_genesis_session, new_session, ContractData, Error as VMError, Session,
    VM,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wallet_core::transaction::{
    moonlight_stake, moonlight_stake_reward, moonlight_unstake,
};

pub mod common;

use crate::common::assert::*;
use crate::common::init::CHAIN_ID;
use crate::common::utils::*;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const STAKE_VALUE: u64 = GENESIS_VALUE / 2;
const GENESIS_NONCE: u64 = 0;

#[test]
fn stake() -> Result<(), VMError> {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut vm = &mut VM::ephemeral()?;
    let mut session = instantiate(&mut vm, &moonlight_pk);

    // ------
    // Stake

    // execute 1st stake transaction
    let stake_1 = STAKE_VALUE / 3;
    let mut nonce = GENESIS_NONCE + 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        stake_1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 1st stake transaction
    let gas_spent_1 = receipt.gas_spent;
    println!("STAKE 1: {gas_spent_1} gas");
    assert_stake_event(&receipt.events, "stake", &stake_pk, stake_1, 0);
    let mut total_stake = stake_1;
    assert_stake(&mut session, &stake_pk, total_stake, 0, 0);
    let mut moonlight_balance = GENESIS_VALUE - stake_1 - gas_spent_1;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Stake top-up before stake is eligible

    // execute 2nd stake transaction
    let stake_2 = STAKE_VALUE / 4;
    nonce += 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        stake_2,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 2nd stake transaction
    let gas_spent_2 = receipt.gas_spent;
    println!("STAKE 2: {gas_spent_2} gas");
    assert_stake_event(&receipt.events, "stake", &stake_pk, stake_2, 0);
    total_stake += stake_2;
    assert_stake(&mut session, &stake_pk, total_stake, 0, 0);
    moonlight_balance -= stake_2;
    moonlight_balance -= gas_spent_2;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Stake top-up after stake is eligible

    // in order to test the locking some of the stake during a top-up, we need
    // to start a new session at a block-height on which the stake is eligible
    let base = session.commit()?;
    let mut session = new_session(&vm, base, CHAIN_ID, 2 * EPOCH)?;

    // execute 3rd stake transaction
    let stake_3 = STAKE_VALUE - stake_1 - stake_2;
    nonce += 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        stake_3,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 3rd stake transaction
    let gas_spent_3 = receipt.gas_spent;
    println!("STAKE 3: {gas_spent_3} gas");
    let locked = stake_3 / 10;
    assert_stake_event(
        &receipt.events,
        "stake",
        &stake_pk,
        stake_3 - locked,
        locked,
    );
    total_stake += stake_3;
    assert_stake(&mut session, &stake_pk, total_stake, locked, 0);
    moonlight_balance -= stake_3;
    moonlight_balance -= gas_spent_3;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Stake top-up fails due to insufficient funds

    nonce += 1;
    let stake_4 = GENESIS_VALUE;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        stake_4,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    assert!(execute(&mut session, tx,).is_err());

    Ok(())
}

#[test]
fn unstake() -> Result<(), VMError> {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut vm = &mut VM::ephemeral()?;
    let mut session = instantiate(&mut vm, &moonlight_pk);

    // initial stake
    let mut nonce = GENESIS_NONCE + 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        STAKE_VALUE,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;
    let mut moonlight_balance = GENESIS_VALUE - STAKE_VALUE - receipt.gas_spent;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Partial Unstake

    // execute 1st unstake transaction
    let unstake_1 = STAKE_VALUE * 2 / 3;
    nonce += 1;
    let tx = moonlight_unstake(
        rng,
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        unstake_1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 1st unstake transaction
    let gas_spent_1 = receipt.gas_spent;
    println!("UNSTAKE 1: {gas_spent_1} gas");
    assert_stake_event(&receipt.events, "unstake", &stake_pk, unstake_1, 0);
    let mut total_stake = STAKE_VALUE - unstake_1;
    assert_stake(&mut session, &stake_pk, total_stake, 0, 0);
    moonlight_balance += unstake_1;
    moonlight_balance -= gas_spent_1;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Partial Unstake with some locked value

    // re-stake the unstaked value after the stake has become eligible
    let base = session.commit()?;
    let mut session = new_session(&vm, base, CHAIN_ID, 2 * EPOCH)?;
    nonce += 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        unstake_1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;
    total_stake = STAKE_VALUE;
    let mut locked = unstake_1 / 10;
    assert_stake(&mut session, &stake_pk, total_stake, locked, 0);
    moonlight_balance -= unstake_1;
    moonlight_balance -= receipt.gas_spent;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // execute 2nd unstake transaction that unstakes everything but 1/3 of the
    // locked stake
    let unstake_from_value = total_stake - locked;
    let unstake_from_locked = locked * 2 / 3;
    let unstake_2 = unstake_from_value + unstake_from_locked;
    nonce += 1;
    let tx = moonlight_unstake(
        rng,
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        unstake_2,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 2nd unstake transaction
    let gas_spent_2 = receipt.gas_spent;
    println!("UNSTAKE 2: {gas_spent_2} gas");
    // only a 3rd of the locked amount should be left
    locked /= 3;
    total_stake = locked;
    assert_stake_event(
        &receipt.events,
        "unstake",
        &stake_pk,
        unstake_from_value,
        unstake_from_locked,
    );
    assert_stake(&mut session, &stake_pk, total_stake, locked, 0);
    moonlight_balance += unstake_2;
    moonlight_balance -= gas_spent_2;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Unstake everything

    // execute 3rd unstake transaction that unstakes the remaining locked amount
    let unstake_3 = locked;
    nonce += 1;
    let tx = moonlight_unstake(
        rng,
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        unstake_3,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 3rd unstake transaction
    let gas_spent_3 = receipt.gas_spent;
    println!("UNSTAKE 3: {gas_spent_2} gas");
    assert_stake_event(&receipt.events, "unstake", &stake_pk, 0, locked);
    assert_stake(&mut session, &stake_pk, 0, 0, 0);
    moonlight_balance += unstake_3;
    moonlight_balance -= gas_spent_3;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    Ok(())
}

#[test]
fn withdraw_reward() -> Result<(), VMError> {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let moonlight_sk = BlsSecretKey::random(rng);
    let moonlight_pk = BlsPublicKey::from(&moonlight_sk);

    let stake_sk = BlsSecretKey::random(rng);
    let stake_pk = BlsPublicKey::from(&stake_sk);

    let mut vm = &mut VM::ephemeral()?;
    let mut session = instantiate(&mut vm, &moonlight_pk);

    // initial stake
    let mut nonce = GENESIS_NONCE + 1;
    let tx = moonlight_stake(
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        STAKE_VALUE,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;
    let mut moonlight_balance = GENESIS_VALUE - STAKE_VALUE - receipt.gas_spent;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);
    // add a reward to the staked key
    const REWARD_AMOUNT: u64 = dusk(3.0);
    add_reward(&mut session, &stake_pk, REWARD_AMOUNT)?;
    assert_stake(&mut session, &stake_pk, STAKE_VALUE, 0, REWARD_AMOUNT);

    // ------
    // Withdraw 1/3 of the reward

    let reward_withdawal_1 = REWARD_AMOUNT / 3;
    nonce += 1;
    let tx = moonlight_stake_reward(
        rng,
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        reward_withdawal_1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 1st reward withdrawal
    let gas_spent_1 = receipt.gas_spent;
    println!("WITHDRAW 1: {gas_spent_1} gas");
    assert_stake_event(
        &receipt.events,
        "withdraw",
        &stake_pk,
        reward_withdawal_1,
        0,
    );
    let mut reward = REWARD_AMOUNT - reward_withdawal_1;
    assert_stake(&mut session, &stake_pk, STAKE_VALUE, 0, reward);
    moonlight_balance += reward_withdawal_1;
    moonlight_balance -= gas_spent_1;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    // ------
    // Withdraw the remaining reward

    let reward_withdawal_2 = reward;
    nonce += 1;
    let tx = moonlight_stake_reward(
        rng,
        &moonlight_sk,
        &stake_sk,
        &stake_sk,
        reward_withdawal_2,
        GAS_LIMIT,
        GAS_PRICE,
        nonce,
        CHAIN_ID,
    )
    .expect("tx creation should pass");
    let receipt = execute(&mut session, tx)?;

    // verify 1st reward withdrawal
    let gas_spent_2 = receipt.gas_spent;
    println!("WITHDRAW 2: {gas_spent_2} gas");
    assert_stake_event(
        &receipt.events,
        "withdraw",
        &stake_pk,
        reward_withdawal_2,
        0,
    );
    reward = 0;
    assert_stake(&mut session, &stake_pk, STAKE_VALUE, 0, reward);
    moonlight_balance += reward_withdawal_2;
    moonlight_balance -= gas_spent_2;
    assert_moonlight(&mut session, &moonlight_pk, moonlight_balance, nonce);

    Ok(())
}

fn add_reward(
    session: &mut Session,
    stake_pk: &BlsPublicKey,
    reward: u64,
) -> Result<(), VMError> {
    let rewards = vec![Reward {
        account: *stake_pk,
        value: reward,
        reason: RewardReason::Other,
    }];

    let receipt =
        session.call::<_, ()>(STAKE_CONTRACT, "reward", &rewards, GAS_LIMIT)?;

    assert_reward_event(&receipt.events, "reward", stake_pk, reward);

    Ok(())
}

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single moonlight account identified by the given public key, owning the
/// genesis-value.
fn instantiate(vm: &mut VM, moonlight_pk: &BlsPublicKey) -> Session {
    // create a new session using an ephemeral vm
    let mut session = new_genesis_session(vm, CHAIN_ID);

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

    let mut session = new_session(vm, base, CHAIN_ID, 1)
        .expect("Instantiating new session should succeed");

    // check that the moonlight account is initialized as expected
    assert_moonlight(&mut session, moonlight_pk, GENESIS_VALUE, GENESIS_NONCE);

    session
}
