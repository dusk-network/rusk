// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use core::cmp::min;

use dusk_bytes::Serializable;

use execution_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    stake::{
        next_epoch, Reward, Stake, StakeAmount, StakeData, StakeEvent,
        StakeWithReceiverEvent, Withdraw, EPOCH, MINIMUM_STAKE, STAKE_CONTRACT,
        STAKE_WARNINGS,
    },
    transfer::TRANSFER_CONTRACT,
};

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
    stakes: BTreeMap<[u8; BlsPublicKey::SIZE], (StakeData, BlsPublicKey)>,
    burnt_amount: u64,
    previous_block_state:
        BTreeMap<[u8; BlsPublicKey::SIZE], (Option<StakeData>, BlsPublicKey)>,
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

        let value = stake.value();
        let account = *stake.account();
        let nonce = stake.nonce();
        let signature = *stake.signature();

        if stake.chain_id() != self.chain_id() {
            panic!("The stake must target the correct chain");
        }

        let loaded_stake = self.load_or_create_stake_mut(&account);

        // ensure the stake is at least the minimum and that there isn't an
        // amount staked already
        if value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        if loaded_stake.amount.is_some() {
            panic!("Can't stake twice for the same key");
        }

        // NOTE: exhausting the nonce is nearly impossible, since it
        //       requires performing more than 18 quintillion stake operations.
        //       Since this number is so large, we also skip overflow checks.
        let incremented_nonce = loaded_stake.nonce + 1;

        // check signature and nonce used are correct
        if nonce != incremented_nonce {
            panic!("Invalid nonce");
        }

        let digest = stake.signature_message().to_vec();
        if !rusk_abi::verify_bls(digest, account, signature) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () =
            rusk_abi::call::<_, ()>(TRANSFER_CONTRACT, "deposit", &value)
                .expect("Depositing funds into contract should succeed");

        // update the state accordingly
        loaded_stake.nonce = nonce;
        loaded_stake.amount =
            Some(StakeAmount::new(value, rusk_abi::block_height()));

        rusk_abi::emit("stake", StakeEvent { account, value });

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((None, account));
    }

    pub fn unstake(&mut self, unstake: Withdraw) {
        self.check_new_block();

        let transfer_withdraw = unstake.transfer_withdraw();
        let account = *unstake.account();
        let value = transfer_withdraw.value();
        let signature = *unstake.signature();

        let loaded_stake = self
            .get_stake_mut(&account)
            .expect("A stake should exist in the map to be unstaked!");
        let prev_stake = Some(*loaded_stake);

        // ensure there is a value staked, and that the withdrawal is exactly
        // the same amount
        let stake = loaded_stake
            .amount
            .as_ref()
            .expect("There must be an amount to unstake");
        let withdrawal_value = stake.locked + stake.value;

        if value != withdrawal_value {
            panic!("Value withdrawn different from staked amount");
        }

        // check signature is correct
        let digest = unstake.signature_message().to_vec();
        if !rusk_abi::verify_bls(digest, account, signature) {
            panic!("Invalid signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "withdraw", transfer_withdraw)
                .expect("Withdrawing stake should succeed");

        // update the state accordingly
        loaded_stake.amount = None;

        rusk_abi::emit(
            "unstake",
            StakeWithReceiverEvent {
                account,
                value: withdrawal_value,
                receiver: Some(*transfer_withdraw.receiver()),
            },
        );

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    pub fn withdraw(&mut self, withdraw: Withdraw) {
        let transfer_withdraw = withdraw.transfer_withdraw();
        let account = *withdraw.account();
        let value = transfer_withdraw.value();
        let signature = *withdraw.signature();

        let loaded_stake = self
            .get_stake_mut(&account)
            .expect("A stake should exist in the map to be unstaked!");

        // ensure there is a non-zero reward, and that the withdrawal is exactly
        // the same amount
        if loaded_stake.reward == 0 {
            panic!("There is no reward available to withdraw");
        }

        if value > loaded_stake.reward {
            panic!("Value withdrawn higher than available reward");
        }

        // check signature is correct
        let digest = withdraw.signature_message().to_vec();
        if !rusk_abi::verify_bls(digest, account, signature) {
            panic!("Invalid signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "mint", transfer_withdraw)
                .expect("Withdrawing reward should succeed");

        // update the state accordingly
        loaded_stake.reward -= value;
        rusk_abi::emit(
            "withdraw",
            StakeWithReceiverEvent {
                account,
                value,
                receiver: Some(*transfer_withdraw.receiver()),
            },
        );
    }

    /// Gets a reference to a stake.
    pub fn get_stake(&self, key: &BlsPublicKey) -> Option<&StakeData> {
        self.stakes.get(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &BlsPublicKey,
    ) -> Option<&mut StakeData> {
        self.stakes.get_mut(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Pushes the given `stake` onto the state for a given `account`.
    pub fn insert_stake(&mut self, account: BlsPublicKey, stake: StakeData) {
        self.stakes.insert(account.to_bytes(), (stake, account));
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_or_create_stake_mut(
        &mut self,
        account: &BlsPublicKey,
    ) -> &mut StakeData {
        let is_missing = self.stakes.get(&account.to_bytes()).is_none();

        if is_missing {
            let stake = StakeData::EMPTY;
            self.stakes.insert(account.to_bytes(), (stake, *account));
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        self.stakes
            .get_mut(&account.to_bytes())
            .map(|(s, _)| s)
            .unwrap()
    }

    /// Rewards a `account` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(&mut self, rewards: Vec<Reward>) {
        // since we assure that reward is called at least once per block, this
        // call is necessary to ensure that there are no inconsistencies in
        // `prev_block_state`.
        self.check_new_block();

        for reward in &rewards {
            let stake = self.load_or_create_stake_mut(&reward.account);

            // Reset faults counters
            stake.faults = 0;
            stake.hard_faults = 0;

            stake.reward += reward.value;
        }

        rusk_abi::emit("reward", rewards);
    }

    /// Total amount burned since the genesis
    pub fn burnt_amount(&self) -> u64 {
        self.burnt_amount
    }

    /// Version of the stake contract
    pub fn get_version(&self) -> u64 {
        STAKE_CONTRACT_VERSION
    }

    /// Slash the given `to_slash` amount from an `account`'s reward
    ///
    /// If the reward is less than the `to_slash` amount, then the reward is
    /// depleted and the provisioner eligibility is shifted to the
    /// next epoch as well
    pub fn slash(&mut self, account: &BlsPublicKey, to_slash: Option<u64>) {
        self.check_new_block();

        let stake = self
            .get_stake_mut(account)
            .expect("The stake to slash should exist");
        let prev_stake = Some(*stake);

        // Stake can have no amount if provisioner unstake in the same block
        if stake.amount.is_none() {
            return;
        }

        stake.faults = stake.faults.saturating_add(1);
        let effective_faults =
            stake.faults.saturating_sub(STAKE_WARNINGS) as u64;

        let stake_amount = stake.amount.as_mut().expect("stake_to_exists");

        // Shift eligibility (aka stake suspension) only if warnings are
        // saturated
        if effective_faults > 0 {
            // The stake is suspended for the rest of the current epoch plus
            // effective_faults epochs
            let to_shift = effective_faults * EPOCH;

            stake_amount.eligibility =
                next_epoch(rusk_abi::block_height()) + to_shift;

            rusk_abi::emit(
                "suspended",
                StakeEvent {
                    account: *account,
                    value: stake_amount.eligibility,
                },
            );
        }

        // Slash the provided amount or calculate the percentage according to
        // effective faults
        let to_slash = to_slash
            .unwrap_or(stake_amount.value / 100 * effective_faults * 10);
        let to_slash = min(to_slash, stake_amount.value);

        if to_slash > 0 {
            stake_amount.lock_amount(to_slash);

            rusk_abi::emit(
                "slash",
                StakeEvent {
                    account: *account,
                    value: to_slash,
                },
            );
        }

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, *account));
    }

    /// Slash the given `to_slash` amount from an `account`'s stake.
    ///
    /// If the stake is less than the `to_slash` amount, then the stake is
    /// depleted
    pub fn hard_slash(
        &mut self,
        account: &BlsPublicKey,
        to_slash: Option<u64>,
        severity: Option<u8>,
    ) {
        self.check_new_block();

        let stake = self
            .get_stake_mut(account)
            .expect("The stake to slash should exist");

        // Stake can have no amount if provisioner unstake in the same block
        if stake.amount.is_none() {
            return;
        }

        let prev_stake = Some(*stake);

        let stake_amount = stake.amount.as_mut().expect("stake_to_exists");

        let severity = severity.unwrap_or(1);
        stake.hard_faults = stake.hard_faults.saturating_add(severity);
        let hard_faults = stake.hard_faults as u64;

        // The stake is shifted (aka suspended) for the rest of the current
        // epoch plus hard_faults epochs
        let to_shift = hard_faults * EPOCH;
        stake_amount.eligibility =
            next_epoch(rusk_abi::block_height()) + to_shift;

        rusk_abi::emit(
            "suspended",
            StakeEvent {
                account: *account,
                value: stake_amount.eligibility,
            },
        );

        // Slash the provided amount or calculate the percentage according to
        // hard faults
        let to_slash =
            to_slash.unwrap_or(stake_amount.value / 100 * hard_faults * 10);
        let to_slash = min(to_slash, stake_amount.value);

        if to_slash > 0 {
            // Update the staked amount
            stake_amount.value -= to_slash;
            Self::deduct_contract_balance(to_slash);

            // Update the total burnt amount
            self.burnt_amount += to_slash;

            rusk_abi::emit(
                "hard_slash",
                StakeEvent {
                    account: *account,
                    value: to_slash,
                },
            );
        }

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, *account));
    }

    /// Sets the burnt amount
    pub fn set_burnt_amount(&mut self, burnt_amount: u64) {
        self.burnt_amount = burnt_amount;
    }

    /// Feeds the host with the stakes.
    pub fn stakes(&self) {
        for (stake_data, account) in self.stakes.values() {
            rusk_abi::feed((*account, *stake_data));
        }
    }

    fn chain_id(&self) -> u8 {
        rusk_abi::chain_id()
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
        for (stake_data, account) in self.previous_block_state.values() {
            rusk_abi::feed((*account, *stake_data));
        }
    }
}
