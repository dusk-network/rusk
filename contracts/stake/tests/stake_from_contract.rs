// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId};
use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_core::stake::{
    Reward, RewardReason, Stake, DEFAULT_MINIMUM_STAKE, STAKE_CONTRACT,
};
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::{Transaction, TRANSFER_CONTRACT};
use dusk_vm::{execute, ContractData, Error, ExecutionConfig, Session, VM};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wallet_core::transaction::moonlight_stake_reward;

pub mod common;
use crate::common::assert::{assert_moonlight, assert_reward_event};
use crate::common::init::CHAIN_ID;
use crate::common::utils::*;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);
const GENESIS_NONCE: u64 = 0;

const OWNER: [u8; 32] = [0; 32];
const ALICE_ID: ContractId = ContractId::from_bytes([3; 32]);
const CHARLIE_ID: ContractId = ContractId::from_bytes([4; 32]);
const REWARD_AMOUNT: u64 = dusk(3.0);

const NO_CONFIG: ExecutionConfig = ExecutionConfig::DEFAULT;

#[test]
fn stake_from_contract() -> Result<(), Error> {
    // ------
    // instantiate the test

    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut VM::ephemeral().expect("Creating ephemeral VM should work");

    let sk_1 = BlsSecretKey::random(rng);
    let pk_1 = BlsPublicKey::from(&sk_1);
    let mut nonce_1 = GENESIS_NONCE;

    let sk_2 = BlsSecretKey::random(rng);
    let pk_2 = BlsPublicKey::from(&sk_2);
    let mut nonce_2 = GENESIS_NONCE;

    let mut session = instantiate(vm, &[pk_1, pk_2]);

    // deploy alice contract to transfer from
    deploy_contract(
        &mut session,
        ALICE_ID,
        include_bytes!(
            "../../../target/wasm32-unknown-unknown/release/alice.wasm"
        ),
    )
    .expect("Deploying the alice contract should succeed");

    // deploy charlie contract to stake from
    deploy_contract(
        &mut session,
        CHARLIE_ID,
        include_bytes!(
            "../../../target/wasm32-unknown-unknown/release/charlie.wasm"
        ),
    )
    .expect("Deploying the charlie contract should succeed");

    // ------
    // add stake reward to second key set

    add_reward(&mut session, &pk_2, REWARD_AMOUNT)?;

    // withdraw half of the reward to verify it's working
    nonce_2 += 1;
    let tx = moonlight_stake_reward(
        rng,
        &sk_2,
        &sk_2,
        &sk_2,
        REWARD_AMOUNT + 1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce_2,
        CHAIN_ID,
    )
    .expect("moonlight tx should be fine");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)?;
    assert_eq!(
        receipt.data.unwrap_err(),
        ContractError::Panic(String::from(
            "Value to withdraw is higher than available reward"
        ))
    );

    // ------
    // Add funds to alice contract

    let _receipt = session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "add_contract_balance",
        &(ALICE_ID, DEFAULT_MINIMUM_STAKE),
        GAS_LIMIT,
    )?;
    assert_eq!(
        DEFAULT_MINIMUM_STAKE,
        session
            .call::<_, u64>(
                TRANSFER_CONTRACT,
                "contract_balance",
                &ALICE_ID,
                GAS_LIMIT
            )
            .unwrap()
            .data,
    );

    // ------
    // Stake from charlie via alice

    let stake_amount = DEFAULT_MINIMUM_STAKE - 1;

    // create stake from contract struct
    let stake =
        Stake::new_from_contract(&sk_1, CHARLIE_ID, stake_amount, CHAIN_ID);
    let contract_call = ContractCall::new(ALICE_ID, "stake_activate")
        .with_args(&stake)
        .expect("Should serialize fn args correctly");
    // let contract_call = ContractCall::new(ALICE_ID, "ping");
    nonce_1 += 1;
    let tx = Transaction::moonlight(
        &sk_1,
        None,
        0,
        stake_amount,
        GAS_LIMIT,
        GAS_PRICE,
        nonce_1,
        CHAIN_ID,
        Some(contract_call),
    )
    .expect("moonlight tx should be fine");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)?;

    let panic_msg = String::from(
        // "Staking to the stake contract via the relayer contract should succeed:
        // Panic(\"Calling receiver should succeed:
        // Panic(\\\"[relayer] Staking to the stake contract should succeed:
        // Panic(\\\\\\\"Calling receiver should succeed:
        // Panic(\\\\\\\\\\\\\\\"The staked value is lower than the minimum amount!\\\\\\\\\\\\\\\")\\\\\\\")\\\")\")"
        // Panic(\\\\\\\\\\\\\\\"The staked value is lower than the minimum amount!\\\\\\\\\\\\\\\")\\\\\\\")\\\")\")"
        "Staking to the stake contract via the relayer contract should succeed: Panic(\"Calling receiver should succeed: Panic(\\\"[relayer] Staking to the stake contract should succeed: Panic(\\\\\\\"Calling receiver should succeed: Panic(\\\\\\\\\\\\\\\"The staked value is lower than the minimum amount!\\\\\\\\\\\\\\\")\\\\\\\")\\\")\")"
    );
    assert_eq!(
        receipt.data.expect_err("The call should result in a panic"),
        ContractError::Panic(panic_msg)
    );

    // withdraw the other half of the reward to verify it's working
    nonce_2 += 1;
    let tx = moonlight_stake_reward(
        rng,
        &sk_2,
        &sk_2,
        &sk_2,
        REWARD_AMOUNT + 1,
        GAS_LIMIT,
        GAS_PRICE,
        nonce_2,
        CHAIN_ID,
    )
    .expect("moonlight tx should be fine");

    let receipt = execute(&mut session, &tx, &NO_CONFIG)?;
    assert_eq!(
        receipt.data.unwrap_err(),
        ContractError::Panic(String::from(
            "Value to withdraw is higher than available reward"
        ))
    );

    Ok(())
}

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single moonlight account identified by the given public key, owning the
/// genesis-value.
fn instantiate(vm: &mut VM, moonlight_pks: &[BlsPublicKey]) -> Session {
    // create a new session using an ephemeral vm
    let mut session = vm.genesis_session(CHAIN_ID);

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

    // insert genesis value to moonlight accounts
    for pk in moonlight_pks {
        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_account_balance",
                &(*pk, GENESIS_VALUE),
                GAS_LIMIT,
            )
            .expect("Inserting genesis account should succeed");

        // check that the moonlight account is initialized as expected
        assert_moonlight(&mut session, pk, GENESIS_VALUE, GENESIS_NONCE);
    }

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    let session = vm
        .session(base, CHAIN_ID, 1)
        .expect("Instantiating new session should succeed");

    session
}

fn add_reward(
    session: &mut Session,
    stake_pk: &BlsPublicKey,
    reward: u64,
) -> Result<(), Error> {
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

fn deploy_contract(
    session: &mut Session,
    id: ContractId,
    bytecode: &[u8],
) -> Result<(), Error> {
    session.deploy(
        bytecode,
        ContractData::builder().owner(OWNER).contract_id(id),
        GAS_LIMIT,
    )?;

    Ok(())
}
