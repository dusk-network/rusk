// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey, Signature};
use dusk_pki::StealthAddress;
use phoenix_core::Note;
use transfer_contract::Call;

#[cfg(not(feature = "no-bridge"))]
mod bridge;

impl StakeContract {
    pub fn stake(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        value: u64,
        proof: Vec<u8>,
    ) {
        if dusk_abi::caller() != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract!");
        }

        if value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        // allot a stake to the given key and increment the signature counter
        let mut stake = self
            .load_mut_stake(&pk)
            .expect("Failed to query state!")
            .expect("The address is not allowed");

        let counter = stake.counter();

        stake.increment_counter();
        stake.insert_amount(value, dusk_abi::block_height());

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify the signature is over the correct message
        let message = Self::stake_sign_message(counter, value);
        let valid_signature = rusk_abi::verify_bls_sign(signature, pk, message);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let transfer = rusk_abi::transfer_contract();
        let transaction = {
            let address = rusk_abi::stake_contract();
            let call =
                Call::send_to_contract_transparent(address, value, proof);
            call.to_transaction()
        };

        dusk_abi::transact_raw(self, &transfer, &transaction, 0)
            .expect("Failed to send note to contract");
    }

    pub fn unstake(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        note: Note,
        withdraw_proof: Vec<u8>,
    ) {
        if dusk_abi::caller() != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract!");
        }

        // remove the stake from a key and increment the signature counter
        let mut stake = self
            .get_stake_mut(&pk)
            .expect("Failed to query the state!")
            .expect("The provided key has no stake!");

        let counter = stake.counter();

        let (value, _) = stake.remove_amount();
        stake.increment_counter();

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify signature
        let message = Self::unstake_sign_message(counter, note);
        let valid_signature = rusk_abi::verify_bls_sign(signature, pk, message);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to withdraw a note from this contract
        // containing the value of the stake
        let transfer = rusk_abi::transfer_contract();
        let transaction =
            Call::withdraw_from_transparent(value, note, withdraw_proof)
                .to_transaction();

        dusk_abi::transact_raw(self, &transfer, &transaction, 0)
            .expect("Failed to withdraw note from contract");
    }

    pub fn withdraw(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        address: StealthAddress,
        nonce: BlsScalar,
    ) {
        if dusk_abi::caller() != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract!");
        }

        // deplete the stake from a key and increment the signature counter
        let mut stake = self
            .get_stake_mut(&pk)
            .expect("Failed to query the state!")
            .expect("The provided key has no stake!");

        let counter = stake.counter();
        let reward = stake.reward();

        if reward == 0 {
            panic!("Nothing to withdraw!");
        }

        stake.deplete_reward();
        stake.increment_counter();

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify signature
        let message = Self::withdraw_sign_message(counter, address, nonce);
        let valid_signature = rusk_abi::verify_bls_sign(signature, pk, message);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to mint the reward to the given
        // address
        let transfer = rusk_abi::transfer_contract();
        let transaction = Call::mint(address, reward, nonce).to_transaction();

        dusk_abi::transact_raw(self, &transfer, &transaction, 0)
            .expect("Failed to mint reward note");
    }

    pub fn allowlist(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        owner: PublicKey,
    ) {
        if dusk_abi::caller() != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract!");
        }

        if self.is_allowlisted(&pk).expect("Failed to query the state") {
            panic!("Address already allowed!");
        }

        if !self.is_owner(&owner).expect("Failed to query the state") {
            panic!("Can only be called by a contract owner!");
        }

        // deplete the stake from a key and increment the signature counter
        let mut owner_stake = self
            .get_stake_mut(&owner)
            .expect("Failed to query the state!")
            .expect("The provided owner has no stake!");

        let owner_counter = owner_stake.counter();

        owner_stake.increment_counter();

        drop(owner_stake);
        // verify signature
        let message = Self::allowlist_sign_message(owner_counter, &pk);
        let valid_signature =
            rusk_abi::verify_bls_sign(signature, owner, message);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        self.insert_allowlist(pk).expect("Failed to allowlist");
    }
}
