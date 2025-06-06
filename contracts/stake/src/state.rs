// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::min;

use dusk_bytes::Serializable;
use dusk_core::abi::{self, ContractId};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    next_epoch, Reward, SlashEvent, Stake, StakeAmount, StakeConfig, StakeData,
    StakeEvent, StakeFundOwner, StakeKeys, Withdraw, WithdrawToContract, EPOCH,
    STAKE_CONTRACT,
};
use dusk_core::transfer::{
    ContractToContract, ReceiveFromContract, TRANSFER_CONTRACT,
};

/// Represents the main state structure for staking operations.
/// Tracks active stakes, burnt amounts, and configurations.
///
/// A caller can stake Dusk, and have it attached to a provisioner's account
/// public key. This stake has a maturation period of minimal one [`EPOCH`],
/// after which it is considered valid and the key eligible to participate in
/// the consensus.
///
/// # Fields
/// - `burnt_amount`: Total amount of tokens burnt due to penalties or other
///   actions.
/// - `config`: Current staking configuration.
/// - `previous_block_state`: State changes from the previous block, indexed by
///   provisioner's account [`BlsPublicKey`].
/// - `stakes: Active stakes, indexed by the provisioner's account
///   [`BlsPublicKey`], including stake data and associated keys.
#[derive(Debug, Default, Clone)]
pub struct StakeState {
    burnt_amount: u64,
    config: StakeConfig,
    previous_block_state:
        BTreeMap<[u8; BlsPublicKey::SIZE], (Option<StakeData>, BlsPublicKey)>,
    stakes: BTreeMap<[u8; BlsPublicKey::SIZE], (StakeData, StakeKeys)>,
}

const STAKE_CONTRACT_VERSION: u64 = 8;

impl StakeState {
    /// Creates a new instance of [`StakeState`] with default values.
    ///
    /// # Returns
    /// A new [`StakeState`] instance with default configurations and empty
    /// state mappings.
    ///
    /// # Note
    /// Ensure to configure the state using [`configure`] before performing any
    /// stake-related operations.
    pub const fn new() -> Self {
        Self {
            burnt_amount: 0u64,
            config: StakeConfig::new(),
            previous_block_state: BTreeMap::new(),
            stakes: BTreeMap::new(),
        }
    }

    /// Returns a reference to the current staking configuration.
    ///
    /// # Returns
    /// A reference to a [`StakeConfig`] object representing the current staking
    /// configuration.
    pub fn config(&self) -> &StakeConfig {
        &self.config
    }

    /// Configures the [`StakeState`] with a new staking configuration.
    ///
    /// # Parameters
    /// - `config`: A [`StakeConfig`] object containing new configuration
    ///   values.
    pub fn configure(&mut self, config: StakeConfig) {
        self.config = config;
    }

    /// Updates the state to reflect changes for a new block.
    ///
    /// This includes processing any pending rewards or penalties and updating
    /// the internal state to prepare for the next block.
    ///
    /// # Note
    /// This method should be called at the end of each block.
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

    /// Stakes an amount for a given account.
    ///
    /// A first time stake needs to mature for at least on [`EPOCH`] before it
    /// becomes eligible.
    /// If there is a mature stake in the state for the provided [`StakeKeys`],
    /// one 10th of the stake top-up will be locked.
    ///
    /// The previous stake for the given provisioner's account is appended to
    /// the `previous_block_state`.
    ///
    /// # Parameters
    /// - `stake`: A [`Stake`] object containing details of the stake.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The provided `chain_id` is incorrect.
    /// - The provisioner's account is stored in the state with a different
    ///   `owner` than the one given in the [`StakeKeys`].
    /// - It's a first time stake for the given provisioner's account and the
    ///   stake is smaller than the configured minimum stake.
    /// - The stake owner is a contract
    /// - One of the provided signatures is invalid.
    /// - There is no deposit set on the transfer-contract or it has a different
    ///   value than the stake amount.
    pub fn stake(&mut self, stake: Stake) {
        let minimum_stake = self.config.minimum_stake;
        let value = stake.value();
        let signature = *stake.signature();

        if stake.chain_id() != self.chain_id() {
            panic!("The stake must target the correct chain");
        }

        let account = stake.keys().account;
        let prev_stake = self.get_stake(&stake.keys().account).copied();
        let (loaded_stake, keys) = self.load_or_create_stake_mut(stake.keys());

        if loaded_stake.amount.is_none() && value < minimum_stake {
            panic!("The staked value is lower than the minimum amount!");
        }

        let owner = Self::unwrap_account_owner(&keys.owner);

        let msg = stake.signature_message().to_vec();
        if !abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let _: () = abi::call::<_, ()>(TRANSFER_CONTRACT, "deposit", &value)
            .expect("Depositing funds into contract should succeed");

        let block_height = abi::block_height();
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
        abi::emit("stake", stake_event);

        let key = keys.account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    /// Processes staking from a smart contract.
    ///
    /// A first time stake needs to mature for at least on [`EPOCH`] before it
    /// becomes eligible.
    /// If there is a mature stake in the state for the provided [`StakeKeys`],
    /// one 10th of the stake top-up will be locked.
    ///
    /// The previous stake for the given provisioner's account is appended to
    /// the `previous_block_state`.
    ///
    /// # Parameters
    /// - `recv`: A [`ReceiveFromContract`] object representing
    ///   contract-specific staking details.
    ///
    ///
    /// # Panics
    /// This function will panic if:
    /// - The `rcvr.data` doesn't deserialize to a [`Stake`] object.
    /// - The `chain_id` of the [`Stake`] is incorrect.
    /// - The provisioner's account is stored in the state with a different
    ///   `owner` than the one given.
    /// - The value in the [`Stake`] object doesn't match the one given in the
    ///   [`ReceiveFromContract`].
    /// - The stake owner is not a contract
    /// - It's a first time stake for the given provisioner's account and the
    ///   stake is smaller than the configured minimum stake or the provided
    ///   signature is incorrect.
    pub fn stake_from_contract(&mut self, recv: ReceiveFromContract) {
        let stake: Stake =
            rkyv::from_bytes(&recv.data).expect("Invalid stake received");
        let value = stake.value();
        let minimum_stake = self.config.minimum_stake;

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
            if value < minimum_stake {
                panic!("The staked value is lower than the minimum amount!");
            }

            // We verify the signature only when there is a new stake
            let signature = stake.signature().account;
            let msg = stake.signature_message().to_vec();
            if !abi::verify_bls(msg, account, signature) {
                panic!("Invalid account signature!");
            }
        }

        let block_height = abi::block_height();
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
        abi::emit("stake", stake_event);

        let key = keys.account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    /// Initiates an unstake request for a given account.
    ///
    /// Any locked amount will be unstaked last.
    ///
    /// The previous stake for the given provisioner's account is appended to
    /// the `previous_block_state`.
    ///
    /// If after this call there is no stake or stake-rewards left for the
    /// provisioner, the stake entry is removed from the state.
    ///
    /// # Parameters
    /// - `unstake: A [`Withdraw`] object containing details of the unstake
    ///   request.
    ///
    /// # Panics
    /// This function will panic if:
    /// - There is no stake for the given provisioner's account or the recorded
    ///   `owner` doesn't match the provided one.
    /// - The requested amount to unstake exceeds the total stake.
    /// - The stake owner is a contract
    /// - One of the provided signatures is invalid.
    /// - The total funds of the stake would be smaller than the configured
    ///   minimum stake after the unstake request.
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
        if !abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () = abi::call(TRANSFER_CONTRACT, "withdraw", transfer_withdraw)
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

        abi::emit("unstake", stake_event);
        if stake.total_funds() == 0 {
            // update the state accordingly
            loaded_stake.amount = None;
            if loaded_stake.reward == 0 {
                self.stakes.remove(&unstake.account().to_bytes());
            }
        } else if stake.total_funds() < self.config.minimum_stake {
            panic!("Stake left is lower than minimum stake");
        }

        let key = account.to_bytes();
        self.previous_block_state
            .entry(key)
            .or_insert((prev_stake, account));
    }

    /// Processes an unstake request originating from a smart contract.
    ///
    /// Any locked amount will be unstaked last.
    ///
    /// The previous stake for the given provisioner's account is appended to
    /// the `previous_block_state`.
    ///
    /// If after this call there is no stake or stake-rewards left for the
    /// provisioner, the stake entry is removed from the state.
    ///
    /// # Parameters
    /// - `unstake`: A [`WithdrawToContract`] object specifying
    ///   contract-specific unstaking details.
    ///
    /// # Panics
    /// This function will panic if:
    /// - There is no stake for the given provisioner's account or the recorded
    ///   `owner` doesn't match the provided one.
    /// - The requested amount to unstake exceeds the total stake.
    /// - The stake owner is not a contract
    /// - The calling contract of this method is not the same as the recorded
    ///   stake `owner`.
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
            abi::caller().expect("unstake must be called by a contract");
        assert!(&caller == owner, "Invalid contract caller");

        let to_contract = ContractToContract {
            contract: caller,
            fn_name: unstake.fn_name().into(),
            value,
            data,
        };

        let _: () =
            abi::call(TRANSFER_CONTRACT, "contract_to_contract", &to_contract)
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

        abi::emit("unstake", stake_event);
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

    /// Withdraw stake rewards owned by a given account.
    //
    /// If after this call there is no stake or stake-rewards left for the
    /// provisioner, the stake entry is removed from the state.
    ///
    /// # Parameters
    /// - `withdraw`: A [`Withdraw`] object containing withdrawal details.
    ///
    /// # Panics
    /// This function will panic if:
    /// - There is no stake for the given provisioner's account or the recorded
    ///   `owner` doesn't match the provided one.
    /// - The value to withdraw is 0.
    /// - The value to withdraw is higher than the accumulated rewards.
    /// - The stake owner is a contract
    /// - One of the provided signatures is incorrect.
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
        if !abi::verify_bls(msg.clone(), owner, signature.owner) {
            panic!("Invalid owner signature!");
        }
        if !abi::verify_bls(msg, keys.account, signature.account) {
            panic!("Invalid account signature!");
        }

        // make call to the transfer contract to withdraw funds from this
        // contract into the receiver specified by the withdrawal.
        let _: () = abi::call(TRANSFER_CONTRACT, "mint", transfer_withdraw)
            .expect("Withdrawing reward should succeed");

        // update the state accordingly
        loaded_stake.reward -= value;
        abi::emit("withdraw", StakeEvent::new(*keys, value));

        if loaded_stake.reward == 0 && loaded_stake.amount.is_none() {
            self.stakes.remove(&account.to_bytes());
        }
    }

    /// Withdraw stake rewards owned by a smart contract.
    //
    /// If after this call there is no stake or stake-rewards left for the
    /// provisioner, the stake entry is removed from the state.
    ///
    /// # Parameters
    /// - `withdraw`: A [`WithdrawToContract`] object specifying withdrawal
    ///   details.
    ///
    /// # Panics
    /// - There is no stake for the given provisioner's account or the recorded
    ///   `owner` doesn't match the provided one.
    /// - The value to withdraw is 0.
    /// - The value to withdraw is higher than the accumulated rewards.
    /// - The stake owner is not a contract
    /// - The calling contract of this method is not the same as the recorded
    ///   stake `owner`.
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
            abi::caller().expect("unstake must be called by a contract");
        assert!(&caller == owner, "Invalid contract caller");

        let to_contract = ContractToContract {
            contract: caller,
            fn_name: withdraw.fn_name().into(),
            value,
            data,
        };

        let _: () =
            abi::call(TRANSFER_CONTRACT, "mint_to_contract", &to_contract)
                .expect("Withdrawing reward to contract should succeed");

        // update the state accordingly
        loaded_stake.reward -= value;
        abi::emit("withdraw", StakeEvent::new(*keys, value));

        if loaded_stake.reward == 0 && loaded_stake.amount.is_none() {
            self.stakes.remove(&account.to_bytes());
        }
    }

    /// Retrieves stake data associated with the provisioner's account
    /// [`BlsPublicKey`].
    ///
    /// # Parameters
    /// - `key`: A reference to a [`BlsPublicKey`] identifying the account.
    ///
    /// # Returns
    /// An Option containing a reference to [`StakeData`] if it exists, or
    /// `None` if not.
    pub fn get_stake(&self, key: &BlsPublicKey) -> Option<&StakeData> {
        self.stakes.get(&key.to_bytes()).map(|(s, _)| s)
    }

    /// Retrieves stake keys associated with a specific provisioner's account
    /// [`BlsPublicKey`].
    ///
    /// # Parameters
    /// - `key`: A reference to a [`BlsPublicKey`].
    ///
    /// # Returns
    /// An Option containing a reference to [`StakeKeys`] if they exist, or
    /// `None` if not.
    pub fn get_stake_keys(&self, key: &BlsPublicKey) -> Option<&StakeKeys> {
        self.stakes.get(&key.to_bytes()).map(|(_, k)| k)
    }

    /// Retrieves a mutable reference to stake data and keys associated with a
    /// specific provisioner's account [`BlsPublicKey`].
    ///
    /// # Parameters
    /// - `key`: A reference to a [`BlsPublicKey`].
    ///
    /// # Returns
    /// An Option containing mutable references to both [`StakeData`] and
    /// [`StakeKeys`], or `None` if not found.
    pub fn get_stake_mut(
        &mut self,
        key: &BlsPublicKey,
    ) -> Option<&mut (StakeData, StakeKeys)> {
        self.stakes.get_mut(&key.to_bytes())
    }

    /// Inserts new stake data and keys into the state.
    ///
    /// Overwrites any existing data for the same [`BlsPublicKey`].
    ///
    /// # Parameters
    /// - `keys`: [`StakeKeys`] associated with the stake.
    /// - `stake`: [`StakeData`] representing the stake details.
    pub fn insert_stake(&mut self, keys: StakeKeys, stake: StakeData) {
        self.stakes.insert(keys.account.to_bytes(), (stake, keys));
    }

    /// Gets a mutable reference to the stake of a given `keys`.
    ///
    /// If said stake doesn't exist, a default one is inserted and a mutable
    /// reference returned.
    ///
    /// # Parameters
    /// - `keys`: [`StakeKeys`] associated with the stake.
    ///
    /// # Panics
    /// Panics if the provided `owner` key doesn't match the existing `owner`
    /// associated to the `account` key.
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

    /// Distributes rewards to multiple stakeholders.
    ///
    /// # Parameters
    /// - `rewards`: A vector of reward details.
    ///
    /// # Panics
    /// Panics if rewards cannot be applied due to invalid data or state
    /// inconsistencies.
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
            abi::emit("reward", rewards);
        }
    }

    /// Returns the total burnt amount in the system.
    ///
    /// # Returns
    /// A `u64` representing the total amount of burnt tokens.
    pub fn burnt_amount(&self) -> u64 {
        self.burnt_amount
    }

    /// Returns the current version of the stake contract.
    ///
    /// # Returns
    /// A `u64` representing the version of the stake state.
    pub fn get_version(&self) -> u64 {
        STAKE_CONTRACT_VERSION
    }

    /// Penalizes a given account by slashing a specified amount.
    ///
    /// If the stake is less than the `to_slash` amount, then the stake is
    /// depleted.
    /// If no `to_slash` amount is given, it is derived from the active stake
    /// and number of `faults`.
    ///
    /// # Parameters
    /// - `account`: A reference to a [`BlsPublicKey`] identifying the account.
    /// - `to_slash`: An optional amount to slash.
    ///
    /// # Panics
    /// Panics if the account does not exist or the slash amount is invalid.
    pub fn slash(&mut self, account: &BlsPublicKey, to_slash: Option<u64>) {
        let stake_warnings = self.config.warnings;
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
            stake.faults.saturating_sub(stake_warnings) as u64;

        let stake_amount = stake.amount.as_mut().expect("stake_to_exists");

        // Shift eligibility (aka stake suspension) only if warnings are
        // saturated
        if effective_faults > 0 {
            // The stake is suspended for the rest of the current epoch plus
            // effective_faults epochs
            let to_shift = effective_faults * EPOCH;

            stake_amount.eligibility =
                next_epoch(abi::block_height()) + to_shift;
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
            abi::emit(
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

    /// Performs a severe penalty by slashing a specified amount of stake from
    /// an account. This function may result in permanent changes to the stake
    /// by burning the slashed tokens if the severity level indicates a high
    /// impact.
    ///
    /// If the stake is less than the `to_slash` amount, then the stake is
    /// depleted
    /// If no `to_slash` amount is given, it is derived from the active stake
    /// and number of `hard_faults`.
    ///
    ///
    /// Parameters:
    /// - `account`: &[`BlsPublicKey`] - The public key of the account to be
    ///   slashed.
    /// - `to_slash`: `Option<u64>` - The amount of stake to slash. If None, a
    ///   default penalty may be applied based on protocol rules.
    /// - `severity`: `Option<u8>` - The severity level of the slash. Higher
    ///   severity could indicate more stringent penalties or different rules.
    ///
    /// Panics:
    /// Panics if the account does not exist in the [`StakeState`] or in the
    /// case of invalid `to_slash` or `severity` values.
    ///
    /// Notes:
    /// This function should be used with caution due to its potential for
    /// significant and irreversible changes to the staking state.
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
        let next_eligibility = next_epoch(abi::block_height()) + to_shift;
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

        abi::emit(
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

    /// Sets the total amount of burnt tokens within the staking state. Burnt
    /// tokens represent a deflationary mechanism within the protocol.
    ///
    /// Parameters:
    /// - `burnt_amount`: `u64` - The new burnt token amount to set.
    ///
    /// Notes:
    /// This value is critical for maintaining accurate tokenomics and should
    /// only be modified by authorized procedures or governance decisions.
    pub fn set_burnt_amount(&mut self, burnt_amount: u64) {
        self.burnt_amount = burnt_amount;
    }

    /// Feeds the host with the current stakes within the state. This function
    /// provides read-only access to all stake entries.
    ///
    /// Notes:
    /// Use this method to gather staking data for analytics, audits, or
    /// protocol decisions.
    pub fn stakes(&self) {
        for (stake_data, account) in self.stakes.values() {
            abi::feed((*account, *stake_data));
        }
    }

    fn chain_id(&self) -> u8 {
        abi::chain_id()
    }

    fn deduct_contract_balance(amount: u64) {
        // Update the module balance to reflect the change in the amount
        // withdrawable from the contract
        let _: () = abi::call(
            TRANSFER_CONTRACT,
            "sub_contract_balance",
            &(STAKE_CONTRACT, amount),
        )
        .expect("Subtracting balance should succeed");
    }

    /// Feeds the host with previous state of the changed provisioners.
    ///
    /// Notes:
    /// This method is essential for understanding recent state transitions and
    /// ensuring the integrity of protocol operations.
    pub fn prev_state_changes(&self) {
        for (stake_data, account) in self.previous_block_state.values() {
            abi::feed((*account, *stake_data));
        }
    }
}
