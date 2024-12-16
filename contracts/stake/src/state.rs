// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use core::cmp::min;

use dusk_bytes::Serializable;

use dusk_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    stake::{
        next_epoch, Reward, SlashEvent, Stake, StakeAmount, StakeData,
        StakeEvent, StakeFundOwner, StakeKeys, Withdraw, WithdrawToContract,
        EPOCH, MINIMUM_STAKE, STAKE_CONTRACT, STAKE_WARNINGS,
    },
    transfer::{ContractToContract, ReceiveFromContract, TRANSFER_CONTRACT},
    ContractId,
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
    stakes: BTreeMap<[u8; BlsPublicKey::SIZE], (StakeData, StakeKeys)>,
    burnt_amount: u64,
    previous_block_state:
        BTreeMap<[u8; BlsPublicKey::SIZE], (Option<StakeData>, BlsPublicKey)>,
}

const STAKE_CONTRACT_VERSION: u64 = 8;

impl StakeState {
    pub const fn new() -> Self {
        Self {
            stakes: BTreeMap::new(),
            burnt_amount: 0u64,
            previous_block_state: BTreeMap::new(),
        }
    }

    pub fn on_new_block(&mut self) {
        self.previous_block_state.clear()
    }

    fn unwrap_account_owner(owner: &StakeFundOwner) -> BlsPublicKey {
        match owner {
            StakeFundOwner::Account(public_key) => {
                assert!(
                    public_key.is_valid(),
                    "Specified owner key is not valid"
                );
                *public_key
            }
            StakeFundOwner::Contract(_) => {
                panic!("expect StakeFundOwner::Account")
            }
        }
    }

    fn unwrap_contract_owner(owner: &StakeFundOwner) -> &ContractId {
        match owner {
            StakeFundOwner::Account(_) => {
                panic!("expect StakeFundOwner::Contract")
            }
            StakeFundOwner::Contract(id) => id,
        }
    }

    pub fn stake(&mut self, stake: Stake) {
        let value = stake.value();
        let signature = *stake.signature();

        if stake.chain_id() != self.chain_id() {
            panic!("The stake must target the correct chain");
        }

        let account = stake.keys().account;
        let prev_stake = self.get_stake(&stake.keys().account).copied();
        let (loaded_stake, keys) = self.load_or_create_stake_mut(stake.keys());

        if loaded_stake.amount.is_none() && value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        let owner = Self::unwrap_account_owner(&keys.owner);

        let msg = stake.signature_message().to_vec();
        if !rusk_abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !rusk_abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () =
            rusk_abi::call::<_, ()>(TRANSFER_CONTRACT, "deposit", &value)
                .expect("Depositing funds into contract should succeed");

        let block_height = rusk_abi::block_height();
        // update the state accordingly
        let stake_event = match &mut loaded_stake.amount {
            Some(amount) => {
                let locked = if block_height >= amount.eligibility {
                    value / 10
                } else {
                    // No penalties applied if the stake is not eligible yet
                    0
                };
                let value = value - locked;
                amount.locked += locked;
                amount.value += value;
                StakeEvent::new(*keys, value).locked(locked)
            }
            amount => {
                let _ = amount.insert(StakeAmount::new(value, block_height));
                StakeEvent::new(*keys, value)
            }
        };
        rusk_abi::emit("stake", stake_event);

        let key = keys.account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    pub fn stake_from_contract(&mut self, recv: ReceiveFromContract) {
        let stake: Stake =
            rkyv::from_bytes(&recv.data).expect("Invalid stake received");
        let value = stake.value();

        if stake.chain_id() != self.chain_id() {
            panic!("The stake must target the correct chain");
        }

        let account = stake.keys().account;
        let prev_stake = self.get_stake(&stake.keys().account).copied();
        let (loaded_stake, keys) = self.load_or_create_stake_mut(stake.keys());

        let contract = Self::unwrap_contract_owner(&keys.owner);
        assert!(contract == &recv.contract, "Invalid contract caller");
        assert!(value == recv.value, "Stake amount mismatch");

        if loaded_stake.amount.is_none() {
            if value < MINIMUM_STAKE {
                panic!("The staked value is lower than the minimum amount!");
            }

            // We verify the signature only when there is a new stake
            let signature = stake.signature().account;
            let msg = stake.signature_message().to_vec();
            if !rusk_abi::verify_bls(msg, account, signature) {
                panic!("Invalid account signature!");
            }
        }

        let block_height = rusk_abi::block_height();
        // update the state accordingly
        let stake_event = match &mut loaded_stake.amount {
            Some(amount) => {
                let locked = if block_height >= amount.eligibility {
                    value / 10
                } else {
                    // No penalties applied if the stake is not eligible yet
                    0
                };
                let value = value - locked;
                amount.locked += locked;
                amount.value += value;
                StakeEvent::new(*keys, value).locked(locked)
            }
            amount => {
                let _ = amount.insert(StakeAmount::new(value, block_height));
                StakeEvent::new(*keys, value)
            }
        };
        rusk_abi::emit("stake", stake_event);

        let key = keys.account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    pub fn unstake(&mut self, unstake: Withdraw) {
        let transfer_withdraw = unstake.transfer_withdraw();
        let account = *unstake.account();
        let value = transfer_withdraw.value();
        let signature = *unstake.signature();

        let (loaded_stake, keys) = self
            .get_stake_mut(&account)
            .expect("A stake should exist in the map to be unstaked!");
        let prev_stake = Some(*loaded_stake);

        // ensure there is a value staked, and that the withdrawal is not
        // greater than the available funds
        let stake = loaded_stake
            .amount
            .as_mut()
            .expect("There must be an amount to unstake");

        if value > stake.total_funds() {
            panic!("Value to unstake higher than the staked amount");
        }

        let owner = Self::unwrap_account_owner(&keys.owner);

        // check signature is correct
        let msg = unstake.signature_message();
        if !rusk_abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !rusk_abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "withdraw", transfer_withdraw)
                .expect("Withdrawing stake should succeed");

        let stake_event = if value > stake.value {
            let from_locked = value - stake.value;
            let from_stake = stake.value;
            stake.value = 0;
            stake.locked -= from_locked;
            StakeEvent::new(*keys, from_stake).locked(from_locked)
        } else {
            stake.value -= value;
            StakeEvent::new(*keys, value)
        };

        rusk_abi::emit("unstake", stake_event);
        if stake.total_funds() == 0 {
            // update the state accordingly
            loaded_stake.amount = None;
            if loaded_stake.reward == 0 {
                self.stakes.remove(&unstake.account().to_bytes());
            }
        } else if stake.total_funds() < MINIMUM_STAKE {
            panic!("Stake left is lower than minimum stake");
        }

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    pub fn unstake_from_contract(&mut self, unstake: WithdrawToContract) {
        let account = unstake.account();
        let value = unstake.value();
        let data = unstake.data().to_vec();

        let (loaded_stake, keys) = self
            .get_stake_mut(account)
            .expect("A stake should exist in the map to be unstaked!");
        let prev_stake = Some(*loaded_stake);

        // ensure there is a value staked, and that the withdrawal is not
        // greater than the available funds
        let stake = loaded_stake
            .amount
            .as_mut()
            .expect("There must be an amount to unstake");

        if value > stake.total_funds() {
            panic!("Value to unstake higher than the staked amount");
        }

        let owner = Self::unwrap_contract_owner(&keys.owner);
        let caller =
            rusk_abi::caller().expect("unstake must be called by a contract");
        assert!(&caller == owner, "Invalid contract caller");

        let to_contract = ContractToContract {
            contract: caller,
            fn_name: unstake.fn_name().into(),
            value,
            data,
        };

        let _: () = rusk_abi::call(
            TRANSFER_CONTRACT,
            "contract_to_contract",
            &to_contract,
        )
        .expect("Unstaking to contract should succeed");

        let stake_event = if value > stake.value {
            let from_locked = value - stake.value;
            let from_stake = stake.value;
            stake.value = 0;
            stake.locked -= from_locked;
            StakeEvent::new(*keys, from_stake).locked(from_locked)
        } else {
            stake.value -= value;
            StakeEvent::new(*keys, value)
        };

        rusk_abi::emit("unstake", stake_event);
        if stake.total_funds() == 0 {
            // update the state accordingly
            loaded_stake.amount = None;
            if loaded_stake.reward == 0 {
                self.stakes.remove(&unstake.account().to_bytes());
            }
        }
        // Note: We no longer enforce the minimum stake condition here to
        // avoid locked funds exploit for contracts.
        /*
            } else if stake.total_funds() < MINIMUM_STAKE {
                panic!("Stake left is lower than minimum stake");
        }
        */

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert_with(|| (prev_stake, *account));
    }

    pub fn withdraw(&mut self, withdraw: Withdraw) {
        let transfer_withdraw = withdraw.transfer_withdraw();
        let account = withdraw.account();
        let value = transfer_withdraw.value();
        let signature = *withdraw.signature();

        let (loaded_stake, keys) = self
            .get_stake_mut(account)
            .expect("A stake should exist in the map to get rewards!");

        // ensure no 0 reward is executed,
        if value == 0 {
            panic!("Withdrawing 0 reward is not allowed");
        }

        // ensure that the withdrawal amount is not greater than the current
        // reward
        if value > loaded_stake.reward {
            panic!("Value to withdraw is higher than available reward");
        }

        let owner = Self::unwrap_account_owner(&keys.owner);

        // check signature is correct
        let msg = withdraw.signature_message();
        if !rusk_abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !rusk_abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "mint", transfer_withdraw)
                .expect("Withdrawing reward should succeed");

        // update the state accordingly
        loaded_stake.reward -= value;
        rusk_abi::emit("withdraw", StakeEvent::new(*keys, value));

        if loaded_stake.reward == 0 && loaded_stake.amount.is_none() {
            self.stakes.remove(&account.to_bytes());
        }
    }

    pub fn withdraw_from_contract(&mut self, withdraw: WithdrawToContract) {
        let account = withdraw.account();
        let value = withdraw.value();
        let data = withdraw.data().to_vec();

        let (loaded_stake, keys) = self
            .get_stake_mut(account)
            .expect("A stake should exist in the map to get rewards!");

        // ensure no 0 reward is executed,
        if value == 0 {
            panic!("Withdrawing 0 reward is not allowed");
        }

        // ensure that the withdrawal amount is not greater than the current
        // reward
        if value > loaded_stake.reward {
            panic!("Value to withdraw is higher than available reward");
        }

        let owner = Self::unwrap_contract_owner(&keys.owner);
        let caller =
            rusk_abi::caller().expect("unstake must be called by a contract");
        assert!(&caller == owner, "Invalid contract caller");

        let to_contract = ContractToContract {
            contract: caller,
            fn_name: withdraw.fn_name().into(),
            value,
            data,
        };

        let _: () =
            rusk_abi::call(TRANSFER_CONTRACT, "mint_to_contract", &to_contract)
                .expect("Withdrawing reward to contract should succeed");

        // update the state accordingly
        loaded_stake.reward -= value;
        rusk_abi::emit("withdraw", StakeEvent::new(*keys, value));

        if loaded_stake.reward == 0 && loaded_stake.amount.is_none() {
            self.stakes.remove(&account.to_bytes());
        }
    }

    /// Gets a reference to a stake.
    pub fn get_stake(&self, key: &BlsPublicKey) -> Option<&StakeData> {
        self.stakes.get(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Gets the keys linked to to a stake.
    pub fn get_stake_keys(&self, key: &BlsPublicKey) -> Option<&StakeKeys> {
        self.stakes.get(&key.to_bytes()).map(|(_, k)| k)
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &BlsPublicKey,
    ) -> Option<&mut (StakeData, StakeKeys)> {
        self.stakes.get_mut(&key.to_bytes())
    }

    /// Pushes the given `stake` onto the state for a given `keys`.
    pub fn insert_stake(&mut self, keys: StakeKeys, stake: StakeData) {
        self.stakes.insert(keys.account.to_bytes(), (stake, keys));
    }

    /// Gets a mutable reference to the stake of a given `keys`.
    ///
    /// If said stake doesn't exist, a default one is inserted and a mutable
    /// reference returned.
    ///
    /// # Panics
    /// Panics if the provided keys doesn't match the existing (if any)
    pub(crate) fn load_or_create_stake_mut(
        &mut self,
        keys: &StakeKeys,
    ) -> &mut (StakeData, StakeKeys) {
        let key = keys.account.to_bytes();

        self.stakes
            .entry(key)
            .and_modify(|(_, loaded_keys)| {
                assert!(keys == loaded_keys, "Keys mismatch")
            })
            .or_insert_with(|| (StakeData::EMPTY, *keys))
    }

    /// Rewards multiple accounts with the given rewards.
    ///
    /// If a stake does not exist in the map, it is skipped.
    pub fn reward(&mut self, rewards: Vec<Reward>) {
        for reward in &rewards {
            let stake =
                if let Some((stake, _)) = self.get_stake_mut(&reward.account) {
                    // Reset faults counters
                    stake.faults = 0;
                    stake.hard_faults = 0;
                    stake
                } else {
                    let keys = StakeKeys::single_key(reward.account);
                    let (stake, _) = self.load_or_create_stake_mut(&keys);
                    stake
                };

            stake.reward += reward.value;
        }
        if !rewards.is_empty() {
            rusk_abi::emit("reward", rewards);
        }
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
        let (stake, _) = self
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
        }

        // Slash the provided amount or calculate the percentage according to
        // effective faults
        let to_slash = to_slash
            .unwrap_or(stake_amount.value / 100 * effective_faults * 10);
        let to_slash = min(to_slash, stake_amount.value);

        if to_slash > 0 {
            stake_amount.lock_amount(to_slash);
        }

        if to_slash > 0 || effective_faults > 0 {
            rusk_abi::emit(
                "slash",
                SlashEvent {
                    account: *account,
                    value: to_slash,
                    next_eligibility: stake_amount.eligibility,
                },
            );
        }

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert_with(|| (prev_stake, *account));
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
        let (stake, _) = self
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
        let next_eligibility = next_epoch(rusk_abi::block_height()) + to_shift;
        stake_amount.eligibility = next_eligibility;

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
        }

        rusk_abi::emit(
            "hard_slash",
            SlashEvent {
                account: *account,
                value: to_slash,
                next_eligibility,
            },
        );

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert_with(|| (prev_stake, *account));
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
