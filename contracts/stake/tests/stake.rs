// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_pki::Ownable;
use microkelvin::{BackendCtor, DiskBackend, Persistence};
use phoenix_core::Note;
use rusk_abi::dusk::*;
use stake_contract::{Stake, StakeContract, MINIMUM_STAKE};
use transfer_circuits::SendToContractTransparentCircuit;
use transfer_wrapper::TransferWrapper;

fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

#[test]
fn stake() {
    Persistence::with_backend(&testbackend(), |_| Ok(()))
        .expect("Backed found");

    let genesis_value = dusk(50_000.0);
    let mut wrapper = TransferWrapper::new(0xbeef, genesis_value);

    let (genesis_ssk, unspent_note) = wrapper.genesis_identifier();
    let (refund_ssk, refund_vk, refund_psk) = wrapper.identifier();
    let (remainder_ssk, remainder_vk, remainder_psk) = wrapper.identifier();

    let block_height = 2;
    let created_at = 1;

    let gas_price = 1;
    let gas_limit = dusk(0.5) / gas_price;
    let stake_value = MINIMUM_STAKE;
    let stake = Stake::new(stake_value, created_at, block_height);

    let stake_secret = SecretKey::random(wrapper.rng());
    let stake_pk = PublicKey::from(&stake_secret);
    let stake_message =
        StakeContract::stake_sign_message(stake_value, created_at);
    let stake_signature =
        stake_secret.sign(&stake_pk, stake_message.as_slice());

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

    let transaction = StakeContract::stake_transaction(
        &fee,
        &crossover,
        blinder,
        stct_signature,
        stake_pk,
        stake_signature,
        stake,
    )
    .expect("Failed to produce stake transaction");

    let eligibility = stake.eligibility();

    let is_staked = wrapper
        .stake_state()
        .is_staked(eligibility, &stake_pk)
        .expect("Failed to query state");

    assert!(!is_staked);

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
        .expect("Failed to stake");

    let is_staked = wrapper
        .stake_state()
        .is_staked(eligibility, &stake_pk)
        .expect("Failed to query state");

    assert!(is_staked);

    let stake_p = wrapper
        .stake_state()
        .get_stake(&stake_pk)
        .expect("Failed to fetch stake");

    assert_eq!(stake, stake_p);

    let block_height = block_height + 1;

    let stakes = wrapper
        .stake_state()
        .stakes()
        .expect("Failed to fetch all stakes");

    assert_eq!(stakes.len(), 1);

    let unspent_note = wrapper
        .notes_owned_by(block_height, &remainder_vk)
        .first()
        .copied()
        .expect("Failed to fetch refund note");

    let (_, withdraw_vk, withdraw_psk) = wrapper.identifier();
    let withdraw_note =
        Note::transparent(wrapper.rng(), &withdraw_psk, stake_value);

    let withdraw_stealth_address = withdraw_note.stealth_address();

    let withdraw_blinder = withdraw_note
        .blinding_factor(None)
        .expect("Decrypt transparent note is infallible");

    let withdraw_message =
        StakeContract::withdraw_sign_message(&stake, &withdraw_note);

    let withdraw_signature =
        stake_secret.sign(&stake_pk, withdraw_message.as_slice());

    let transaction = StakeContract::withdraw_transaction(
        stake_pk,
        withdraw_signature,
        withdraw_note,
        stake_value,
        withdraw_blinder,
    )
    .expect("Failed to produce withdraw transaction");

    let eligibility = stake.eligibility();

    let is_staked = wrapper
        .stake_state()
        .is_staked(eligibility, &stake_pk)
        .expect("Failed to query state");

    assert!(is_staked);

    let gas_limit = dusk(1.5) / gas_price;
    let (fee, crossover) =
        wrapper.fee_crossover(gas_limit, gas_price, &refund_psk, stake_value);
    wrapper
        .execute(
            block_height,
            &[unspent_note],
            &[remainder_ssk],
            &refund_vk,
            &remainder_psk,
            true,
            fee,
            Some(crossover),
            Some(transaction),
        )
        .expect("Failed to withdraw");

    let is_staked = wrapper
        .stake_state()
        .is_staked(eligibility, &stake_pk)
        .expect("Failed to query state");

    assert!(!is_staked);

    let note = wrapper
        .notes_owned_by(block_height, &withdraw_vk)
        .first()
        .copied()
        .expect("Failed to fetch withdraw note");

    let stealth_address = note.stealth_address();

    assert_eq!(withdraw_stealth_address, stealth_address);

    let value = note
        .value(None)
        .expect("Failed to decrypt transparent note");

    assert_eq!(stake_value, value);
}
