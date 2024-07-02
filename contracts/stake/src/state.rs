// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use core::cmp::min;

use dusk_bytes::Serializable;

use execution_core::{
    stake::{
        next_epoch, Stake, StakeData, StakingEvent, Unstake, Withdraw, EPOCH,
        STAKE_WARNINGS,
    },
    transfer::Mint,
    StakePublicKey,
};
use rusk_abi::{STAKE_CONTRACT, TRANSFER_CONTRACT};

use crate::*;

/// Contract keeping track of each public key's stake.
///
/// A caller can stake Dusk, and have it attached to a public key. This stake
/// has a maturation period, after which it is considered valid and the key
/// eligible to participate in the consensus.
///
/// Rewards may be received by a public key regardless of whether they have a
/// valid stake.
#[derive(Debug, Default, Clone)]
pub struct StakeState {
    stakes: BTreeMap<[u8; StakePublicKey::SIZE], (StakeData, StakePublicKey)>,
    burnt_amount: u64,
    previous_block_state: BTreeMap<
        [u8; StakePublicKey::SIZE],
        (Option<StakeData>, StakePublicKey),
    >,
    // This is needed just to keep track of blocks to automatically clear the
    // prev_block_state. Future implementations will rely on
    // `before_state_transition` to handle that
    previous_block_height: u64,
}

const STAKE_CONTRACT_VERSION: u64 = 8;

impl StakeState {
    pub const fn new() -> Self {
        Self {
            stakes: BTreeMap::new(),
            burnt_amount: 0u64,
            previous_block_state: BTreeMap::new(),
            previous_block_height: 0,
        }
    }

    pub fn on_new_block(&mut self) {
        self.previous_block_state.clear()
    }

    fn check_new_block(&mut self) {
        let current_height = rusk_abi::block_height();
        if current_height != self.previous_block_height {
            self.previous_block_height = current_height;
            self.on_new_block();
        }
    }

    pub fn stake(&mut self, stake: Stake) {
        self.check_new_block();

        if stake.value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        // allot a stake to the given key and increment the signature counter
        let loaded_stake = self.load_or_create_stake_mut(&stake.public_key);

        let counter = loaded_stake.counter();

        loaded_stake.increment_counter();
        loaded_stake.insert_amount(stake.value, rusk_abi::block_height());

        // verify the signature is over the correct digest
        let digest = Stake::signature_message(counter, stake.value).to_vec();

        if !rusk_abi::verify_bls(digest, stake.public_key, stake.signature) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () = rusk_abi::call(TRANSFER_CONTRACT, "deposit", &stake.value)
            .expect("Depositing funds into contract should succeed");

        rusk_abi::emit(
            "stake",
            StakingEvent {
                public_key: stake.public_key,
                value: stake.value,
            },
        );

        let key = stake.public_key.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((None, stake.public_key));
    }

    pub fn unstake(&mut self, unstake: Unstake) {
        self.check_new_block();

        // remove the stake from a key and increment the signature counter
        let loaded_stake = self
            .get_stake_mut(&unstake.public_key)
            .expect("A stake should exist in the map to be unstaked!");

        let prev_value = Some(loaded_stake.clone());

        let counter = loaded_stake.counter();

        let (value, _) = loaded_stake.remove_amount();
        loaded_stake.increment_counter();

        // verify signature
        let digest =
            Unstake::signature_message(counter, value, unstake.address)
                .to_vec();

        if !rusk_abi::verify_bls(digest, unstake.public_key, unstake.signature)
        {
            panic!("Invalid signature!");
        }
        // make call to transfer contract to withdraw a note from this contract
        // containing the value of the stake
        let withdraw_data = Mint {
            value,
            address: unstake.address,
            sender: unstake.public_key,
        };
        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "withdraw", &withdraw_data)
                .expect(
                "Withdrawing the balance from contract should be successful",
            );

        rusk_abi::emit(
            "unstake",
            StakingEvent {
                public_key: unstake.public_key,
                value,
            },
        );

        let key = unstake.public_key.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_value, unstake.public_key));
    }

    pub fn withdraw(&mut self, withdraw: Withdraw) {
        // deplete the stake from a key and increment the signature counter
        let loaded_stake = self
            .get_stake_mut(&withdraw.public_key)
            .expect("A stake should exist in the map to be withdrawn!");

        let counter = loaded_stake.counter();
        let reward = loaded_stake.reward();

        if reward == 0 {
            panic!("Nothing to withdraw!");
        }

        loaded_stake.deplete_reward();
        loaded_stake.increment_counter();

        // verify signature
        let digest = Withdraw::signature_message(
            counter,
            withdraw.address,
            withdraw.nonce,
        )
        .to_vec();

        if !rusk_abi::verify_bls(
            digest,
            withdraw.public_key,
            withdraw.signature,
        ) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to mint the reward to the given
        // address
        let transfer_contract = TRANSFER_CONTRACT;
        let _: bool = rusk_abi::call(
            transfer_contract,
            "mint",
            &Mint {
                address: withdraw.address,
                value: reward,
                sender: withdraw.public_key,
            },
        )
        .expect("Minting a reward note should succeed");

        rusk_abi::emit(
            "withdraw",
            StakingEvent {
                public_key: withdraw.public_key,
                value: reward,
            },
        );
    }

    /// Gets a reference to a stake.
    pub fn get_stake(&self, key: &StakePublicKey) -> Option<&StakeData> {
        self.stakes.get(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &StakePublicKey,
    ) -> Option<&mut StakeData> {
        self.stakes.get_mut(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Pushes the given `stake` onto the state for a given `stake_pk`.
    pub fn insert_stake(&mut self, stake_pk: StakePublicKey, stake: StakeData) {
        self.stakes.insert(stake_pk.to_bytes(), (stake, stake_pk));
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_or_create_stake_mut(
        &mut self,
        stake_pk: &StakePublicKey,
    ) -> &mut StakeData {
        let is_missing = self.stakes.get(&stake_pk.to_bytes()).is_none();

        if is_missing {
            let stake = StakeData::default();
            self.stakes.insert(stake_pk.to_bytes(), (stake, *stake_pk));
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        self.stakes
            .get_mut(&stake_pk.to_bytes())
            .map(|(s, _)| s)
            .unwrap()
    }

    /// Rewards a `stake_pk` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(&mut self, stake_pk: &StakePublicKey, value: u64) {
        self.check_new_block();

        let stake = self.load_or_create_stake_mut(stake_pk);
        // Reset faults counter
        stake.faults = 0;
        stake.increase_reward(value);
        rusk_abi::emit(
            "reward",
            StakingEvent {
                public_key: *stake_pk,
                value,
            },
        );
    }

    /// Total amount burned since the genesis
    pub fn burnt_amount(&self) -> u64 {
        self.burnt_amount
    }

    /// Version of the stake contract
    pub fn get_version(&self) -> u64 {
        STAKE_CONTRACT_VERSION
    }

    /// Slash the given `to_slash` amount from a `stake_pk` reward
    ///
    /// If the reward is less than the `to_slash` amount, then the reward is
    /// depleted and the provisioner eligibility is shifted to the
    /// next epoch as well
    pub fn slash(&mut self, stake_pk: &StakePublicKey, to_slash: Option<u64>) {
        self.check_new_block();

        let stake = self
            .get_stake_mut(stake_pk)
            .expect("The stake to slash should exist");

        // Stake can have no amount if provisioner unstake in the same block
        if stake.amount().is_none() {
            return;
        }

        let prev_value = Some(stake.clone());

        stake.faults = stake.faults.saturating_add(1);
        let effective_faults =
            stake.faults.saturating_sub(STAKE_WARNINGS) as u64;

        let (stake_amount, eligibility) =
            stake.amount.as_mut().expect("stake_to_exists");

        // Shift eligibility (aka stake suspension) only if warnings are
        // saturated
        if effective_faults > 0 {
            // The stake is suspended for the rest of the current epoch plus
            // effective_faults epochs
            let to_shift = effective_faults * EPOCH;
            *eligibility = next_epoch(rusk_abi::block_height()) + to_shift;
            rusk_abi::emit(
                "suspended",
                StakingEvent {
                    public_key: *stake_pk,
                    value: *eligibility,
                },
            );
        }

        // Slash the provided amount or calculate the percentage according to
        // effective faults
        let to_slash =
            to_slash.unwrap_or(*stake_amount / 100 * effective_faults * 10);
        let to_slash = min(to_slash, *stake_amount);

        if to_slash > 0 {
            // Move the slash amount from stake to reward and deduct contract
            // balance
            *stake_amount -= to_slash;
            stake.increase_reward(to_slash);
            Self::deduct_contract_balance(to_slash);

            rusk_abi::emit(
                "slash",
                StakingEvent {
                    public_key: *stake_pk,
                    value: to_slash,
                },
            );
        }

        let key = stake_pk.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_value, *stake_pk));
    }

    /// Slash the given `to_slash` amount from a `stake_pk` stake
    ///
    /// If the stake is less than the `to_slash` amount, then the stake is
    /// depleted
    pub fn hard_slash(&mut self, stake_pk: &StakePublicKey, to_slash: u64) {
        self.check_new_block();

        let stake_info = self
            .get_stake_mut(stake_pk)
            .expect("The stake to slash should exist");

        let prev_value = Some(stake_info.clone());

        let stake = stake_info.amount.as_mut();
        // This can happen if the provisioner unstake in the same block
        if stake.is_none() {
            return;
        }

        let stake = stake.expect("The stake amount to slash should exist");

        let to_slash = min(to_slash, stake.0);
        if to_slash == 0 {
            return;
        }

        // Update the staked amount
        stake.0 -= to_slash;

        Self::deduct_contract_balance(to_slash);

        // Update the total burnt amount
        self.burnt_amount += to_slash;

        rusk_abi::emit(
            "hard_slash",
            StakingEvent {
                public_key: *stake_pk,
                value: to_slash,
            },
        );
        let key = stake_pk.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_value, *stake_pk));
    }

    /// Sets the burnt amount
    pub fn set_burnt_amount(&mut self, burnt_amount: u64) {
        self.burnt_amount = burnt_amount;
    }

    /// Feeds the host with the stakes.
    pub fn stakes(&self) {
        for (stake_data, stake_pk) in self.stakes.values() {
            rusk_abi::feed((*stake_pk, stake_data.clone()));
        }
    }

    fn deduct_contract_balance(amount: u64) {
        // Update the module balance to reflect the change in the amount
        // withdrawable from the contract
        let _: () = rusk_abi::call(
            TRANSFER_CONTRACT,
            "sub_contract_balance",
            &(STAKE_CONTRACT, amount),
        )
        .expect("Subtracting balance should succeed");
    }

    /// Feeds the host with previous state of the changed provisioners.
    pub fn prev_state_changes(&self) {
        for (stake_data, stake_pk) in self.previous_block_state.values() {
            rusk_abi::feed((*stake_pk, stake_data.clone()));
        }
    }
}
