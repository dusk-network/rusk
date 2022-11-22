// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_jubjub::JubJubScalar;
use phoenix_core::{Fee, Note};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::*;
use stake_contract::{Stake, StakeState, MINIMUM_STAKE};
// use transfer_circuits::SendToContractTransparentCircuit;
use transfer_wrapper::{StakeState, TransferWrapper};

use dusk_pki::{PublicSpendKey, SecretSpendKey};
use piecrust::{Session, VM};

// fn testbackend() -> BackendCtor<DiskBackend> {
//     BackendCtor::new(DiskBackend::ephemeral)
// }

// #[test]
// fn withdraw() {
//     Persistence::with_backend(&testbackend(), |_| Ok(()))
//         .expect("Backend found");
//
//     let mut rng = StdRng::seed_from_u64(0xbeef);
//
//     let sk = SecretKey::random(&mut rng);
//     let pk = PublicKey::from(&sk);
//
//     let reward_value = dusk(1000.0);
//     let genesis_value = dusk(50_000.0);
//     let block_height = 1;
//
//     let gas_price = 1;
//     let gas_limit = dusk(1.0) / gas_price;
//
//     let stake = Stake::new(0, reward_value, block_height);
//     let stake = StakeState {
//         stakes: &[(pk, stake)],
//         owners: &[],
//         allowlist: &[],
//     };
//     let mut wrapper =
//         TransferWrapper::with_stakes(0xbeef, genesis_value, stake);
//
//     let (genesis_ssk, unspent_note) = wrapper.genesis_identifier();
//     let (_, refund_vk, _) = wrapper.identifier();
//     let (_, _, remainder_psk) = wrapper.identifier();
//     let (_, withdraw_vk, withdraw_psk) = wrapper.identifier();
//
//     let fee = Fee::new(&mut rng, gas_limit, gas_price, &remainder_psk);
//
//     let withdraw_r = JubJubScalar::random(&mut rng);
//     let withdraw_address = withdraw_psk.gen_stealth_address(&withdraw_r);
//     let withdraw_nonce = BlsScalar::random(&mut rng);
//
//     let withdraw_msg = StakeContract::withdraw_sign_message(
//         0,
//         withdraw_address,
//         withdraw_nonce,
//     );
//     let withdraw_sig = sk.sign(&pk, &withdraw_msg);
//
//     let transaction = StakeContract::withdraw_transaction(
//         pk,
//         withdraw_sig,
//         withdraw_address,
//         withdraw_nonce,
//     );
//
//     wrapper
//         .execute(
//             block_height,
//             &[unspent_note],
//             &[genesis_ssk],
//             &refund_vk,
//             &remainder_psk,
//             true,
//             fee,
//             None,
//             Some(transaction),
//         )
//         .expect("Failed to execute withdraw transaction");
//
//     let notes = wrapper.notes_owned_by(0, &withdraw_vk);
//
//     assert_eq!(notes.len(), 1);
//
//     let withdraw_value = notes[0]
//         .value(None)
//         .expect("Reward note should be transparent");
//     assert_eq!(
//         withdraw_value, reward_value,
//         "Reward withdrawn should be consistent"
//     );
//
//     let stake_contract = wrapper.stake_state();
//     let stake = stake_contract
//         .get_stake(&pk)
//         .expect("Failed querying the state")
//         .expect("Stake should still exist after withdraw");
//
//     assert_eq!(stake.reward(), 0, "Remaining reward should be 0");
//     assert_eq!(stake.counter(), 1, "Counter should be incremented");
// }

// #[test]
// fn unstake() {
//     Persistence::with_backend(&testbackend(), |_| Ok(()))
//         .expect("Backend found");
//
//     let mut rng = StdRng::seed_from_u64(0xbeef);
//
//     let sk = SecretKey::random(&mut rng);
//     let pk = PublicKey::from(&sk);
//
//     let stake_value = dusk(10_000.0);
//     let genesis_value = dusk(50_000.0);
//     let block_height = 1;
//
//     let gas_price = 1;
//     let gas_limit = dusk(1.1) / gas_price;
//
//     let stake = Stake::new(stake_value, 0, block_height);
//     let stake = StakeState {
//         stakes: &[(pk, stake)],
//         owners: &[],
//         allowlist: &[],
//     };
//
//     let mut wrapper =
//         TransferWrapper::with_stakes(0xbeef, genesis_value, stake);
//     let (genesis_ssk, unspent_note) = wrapper.genesis_identifier();
//     let (_, refund_vk, refund_psk) = wrapper.identifier();
//     let (_, _, remainder_psk) = wrapper.identifier();
//     let (_, unstake_vk, unstake_psk) = wrapper.identifier();
//
//     let (fee, crossover) =
//         wrapper.fee_crossover(gas_limit, gas_price, &refund_psk,
// stake_value);
//
//     let unstake_note = Note::transparent(&mut rng, &unstake_psk,
// stake_value);     let unstake_blinder = unstake_note
//         .blinding_factor(None)
//         .expect("Decrypt transparent note is infallible");
//
//     let unstake_message = StakeContract::unstake_sign_message(0,
// unstake_note);     let unstake_sig = sk.sign(&pk, &unstake_message);
//
//     let transaction = StakeContract::unstake_transaction(
//         pk,
//         unstake_sig,
//         unstake_note,
//         unstake_blinder,
//     )
//     .expect("Failed to produce withdraw transaction");
//
//     wrapper
//         .execute(
//             block_height,
//             &[unspent_note],
//             &[genesis_ssk],
//             &refund_vk,
//             &remainder_psk,
//             true,
//             fee,
//             Some(crossover),
//             Some(transaction),
//         )
//         .expect("Failed to execute unstake transaction");
//
//     let notes = wrapper.notes_owned_by(block_height, &unstake_vk);
//
//     assert_eq!(notes.len(), 1);
//
//     let unstake_value = notes[0]
//         .value(None)
//         .expect("Unstake note should be transparent");
//     assert_eq!(
//         unstake_value, stake_value,
//         "Unstake value should be consistent"
//     );
//
//     let stake_contract = wrapper.stake_state();
//     let stake = stake_contract
//         .get_stake(&pk)
//         .expect("Failed querying the state")
//         .expect("Stake should still exist after unstake");
//
//     assert_eq!(stake.amount(), None, "There should be no stake amount");
//     assert_eq!(stake.counter(), 1, "Counter should be incremented");
// }

const GENESIS_VALUE: u64 = 1_000;
const POINT_LIMIT: u64 = 0x700000;

fn instantiate(vm: &mut VM) -> (SecretSpendKey, PublicSpendKey, Session) {
    let bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );

    let mut session = vm.session();
    session.set_point_limit(POINT_LIMIT);

    let transfer_id = rusk_abi::transfer_module();

    session
        .deploy_with_id(transfer_id, bytecode)
        .expect("Deploying the transfer contract should succeed");

    let mut rng = StdRng::seed_from_u64(0xbeef);

    let ssk = SecretSpendKey::random(&mut rng);
    let psk = PublicSpendKey::from(&ssk);

    let genesis_note = Note::transparent(&mut rng, &psk, GENESIS_VALUE);

    // push genesis note to the contract
    let _: Note = session
        .transact(transfer_id, "push_note", (0u64, genesis_note))
        .expect("Pushing genesis note should succeed");

    println!("points spent: {}", session.spent());

    (ssk, psk, session)
}

#[test]
fn stake() {
    let mut vm = VM::ephemeral().expect("Creating a VM should succeed");

    let (_ssk, _psk, mut _session) = instantiate(&mut vm);

    let mut rng = StdRng::seed_from_u64(0xbeef);

    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    let genesis_value = dusk(50_000.0);
    let stake = StakeState {
        stakes: &[],
        owners: &[],
        allowlist: &[pk],
    };

    let mut wrapper =
        TransferWrapper::with_stakes(0xbeef, genesis_value, stake);

    let (genesis_ssk, unspent_note) = wrapper.genesis_identifier();
    let (refund_ssk, refund_vk, refund_psk) = wrapper.identifier();
    let (_, _, remainder_psk) = wrapper.identifier();

    let block_height = 2;

    let gas_price = 1;
    let gas_limit = dusk(1.5) / gas_price;
    let stake_value = MINIMUM_STAKE;

    let stake_message = StakeState::stake_sign_message(0, stake_value);

    let stake_signature = sk.sign(&pk, stake_message.as_slice());

    let (fee, crossover) =
        wrapper.fee_crossover(gas_limit, gas_price, &refund_psk, stake_value);
    let blinder =
        TransferWrapper::decrypt_blinder(&fee, &crossover, &refund_vk);

    let address = rusk_abi::stake_contract();
    let address = rusk_abi::contract_to_scalar(&address);
    let stct_signature = SendToContractTransparentCircuit::sign(
        wrapper.rng(),
        &refund_ssk,
        &fee,
        &crossover,
        stake_value,
        &address,
    );

    let transaction = StakeState::stake_transaction(
        &fee,
        &crossover,
        blinder,
        stct_signature,
        pk,
        stake_signature,
        stake_value,
    )
    .expect("Failed to produce stake transaction");

    wrapper
        .execute(
            block_height,
            &[unspent_note],
            &[genesis_ssk],
            &refund_vk,
            &remainder_psk,
            true,
            fee,
            Some(crossover),
            Some(transaction),
        )
        .expect("Failed to execute stake transaction");

    let stake_contract = wrapper.stake_state();
    let stake = stake_contract
        .get_stake(&pk)
        .expect("Failed querying the state")
        .expect("Stake should exist after stake");

    let (staked_value, eligibility) =
        stake.amount().expect("Stake should have an amount");

    assert_eq!(
        stake_value, *staked_value,
        "The staked amount should be consistent"
    );
    assert_eq!(
        *eligibility,
        Stake::eligibility_from_height(block_height),
        "Eligibility should be as expected"
    );
    assert_eq!(stake.counter(), 1, "Counter should be incremented");
}

// #[test]
// fn allowlist() {
//     Persistence::with_backend(&testbackend(), |_| Ok(()))
//         .expect("Backend found");
//
//     let mut rng = StdRng::seed_from_u64(0xbeef);
//
//     let sk_owner = SecretKey::random(&mut rng);
//     let pk_owner = PublicKey::from(&sk_owner);
//
//     let genesis_value = dusk(50_000.0);
//     let stake = StakeState {
//         stakes: &[],
//         owners: &[pk_owner],
//         allowlist: &[],
//     };
//
//     let mut wrapper =
//         TransferWrapper::with_stakes(0xbeef, genesis_value, stake);
//
//     let (genesis_ssk, unspent_note) = wrapper.genesis_identifier();
//     let (_, refund_vk, refund_psk) = wrapper.identifier();
//     let (_, _, remainder_psk) = wrapper.identifier();
//
//     let block_height = 2;
//
//     let gas_price = 1;
//     let gas_limit = dusk(1.5) / gas_price;
//     let stake_value = MINIMUM_STAKE;
//
//     let sk = SecretKey::random(&mut rng);
//     let pk = PublicKey::from(&sk);
//
//     let allowlist_message = StakeContract::allowlist_sign_message(0, &pk);
//
//     let allowlist_signature =
//         sk_owner.sign(&pk_owner, allowlist_message.as_slice());
//
//     let (fee, crossover) =
//         wrapper.fee_crossover(gas_limit, gas_price, &refund_psk,
// stake_value);
//
//     let transaction =
//         StakeContract::allowlist_transaction(pk, allowlist_signature,
// pk_owner);
//
//     wrapper
//         .execute(
//             block_height,
//             &[unspent_note],
//             &[genesis_ssk],
//             &refund_vk,
//             &remainder_psk,
//             true,
//             fee,
//             Some(crossover),
//             Some(transaction),
//         )
//         .expect("Failed to execute stake transaction");
//
//     let stake_contract = wrapper.stake_state();
//
//     let list = stake_contract
//         .stakers_allowlist()
//         .expect("Failed to query the original state");
//
//     assert!(list.contains(&pk));
//
//     let stake = stake_contract
//         .get_stake(&pk)
//         .expect("Failed querying the state");
//     assert!(
//         stake.is_none(),
//         "Adding to allowlist should not create the stake"
//     );
//
//     let stake_owner = stake_contract
//         .get_stake(&pk_owner)
//         .expect("Failed querying the state")
//         .expect("Owner stake should still exist after a allowlist");
//     assert_eq!(stake_owner.counter(), 1, "Counter should be incremented");
//
//     let allowlisted = stake_contract
//         .stakers_allowlist()
//         .expect("Failed querying the state");
//     assert!(allowlisted.contains(&pk), "provisioner should be allowed");
// }
