// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use rusk_abi::State;

use phoenix_core::transaction::*;

type BlockHeight = u64;

/// Maturity of the stake
const MATURITY: u64 = 2 * EPOCH;

/// Epoch used for stake operations
const EPOCH: u64 = 2160;

#[derive(Debug, Clone)]
pub struct StakeDataWrapper(pub StakeData);

impl StakeDataWrapper {
    /// Returns the value of the reward.
    #[must_use]
    pub const fn reward(&self) -> u64 {
        self.0.reward
    }

    /// Returns the interaction count of the stake.
    #[must_use]
    pub const fn counter(&self) -> u64 {
        self.0.counter
    }

    /// Insert a stake [`amount`] with a particular `value`, starting from a
    /// particular `block_height`.
    ///
    /// # Panics
    /// If the value is zero or the stake already contains an amount.
    pub fn insert_amount(&mut self, value: u64, block_height: BlockHeight) {
        assert_ne!(value, 0, "A stake can't have zero value");
        assert!(
            self.0.amount.is_none(),
            "Can't stake twice for the same key!"
        );

        let eligibility = Self::eligibility_from_height(block_height);
        self.0.amount = Some((value, eligibility));
    }

    /// Increases the held reward by the given `value`.
    pub fn increase_reward(&mut self, value: u64) {
        self.0.reward += value;
    }

    /// Removes the total [`amount`] staked.
    ///
    /// # Panics
    /// If the stake has no amount.
    pub fn remove_amount(&mut self) -> (u64, BlockHeight) {
        self.0
            .amount
            .take()
            .expect("Can't withdraw non-existing amount!")
    }

    /// Sets the reward to zero.
    pub fn deplete_reward(&mut self) {
        self.0.reward = 0;
    }

    /// Increment the interaction [`counter`].
    pub fn increment_counter(&mut self) {
        self.0.counter += 1;
    }

    /// Compute the eligibility of a stake from the starting block height.
    ///
    /// A stake is eligible to participate in the consensus two EPOCHs
    /// (MATURITY) after the end of the current one.
    #[must_use]
    pub const fn eligibility_from_height(block_height: BlockHeight) -> u64 {
        let epoch = EPOCH - block_height % EPOCH;
        block_height + epoch + MATURITY
    }
}

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
    stakes: BTreeMap<[u8; PublicKey::SIZE], StakeDataWrapper>,
    allowlist: BTreeSet<[u8; PublicKey::SIZE]>,
    owners: BTreeSet<[u8; PublicKey::SIZE]>,
}

impl StakeState {
    pub const fn new() -> Self {
        Self {
            stakes: BTreeMap::new(),
            allowlist: BTreeSet::new(),
            owners: BTreeSet::new(),
        }
    }

    pub fn stake(self: &mut State<Self>, stake: Stake) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        if !self.is_allowlisted(&stake.public_key) {
            panic!("The address is not allowed!");
        }

        if stake.value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        // allot a stake to the given key and increment the signature counter
        let loaded_stake = self.load_stake_mut(&stake.public_key);

        let counter = loaded_stake.counter();

        loaded_stake.increment_counter();
        loaded_stake.insert_amount(stake.value, rusk_abi::block_height());

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(loaded_stake);

        // verify the signature is over the correct digest
        let digest = stake_signature_message(counter, stake.value).to_vec();

        if !rusk_abi::verify_bls(digest, stake.public_key, stake.signature) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let transfer_module = rusk_abi::transfer_module();

        let stct = Stct {
            module: rusk_abi::self_id().to_bytes(),
            value: stake.value,
            proof: stake.proof,
        };

        let _: bool = self
            .transact(transfer_module, "stct", &stct)
            .expect("Sending note to contract should succeed");
    }

    pub fn unstake(self: &mut State<Self>, unstake: Unstake) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        // remove the stake from a key and increment the signature counter
        let loaded_stake = self
            .get_stake_mut(&unstake.public_key)
            .expect("A stake should exist in the map to be unstaked!");

        let counter = loaded_stake.counter();

        let (value, _) = loaded_stake.remove_amount();
        loaded_stake.increment_counter();

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(loaded_stake);

        // verify signature
        let digest = unstake_signature_message(counter, unstake.note).to_vec();

        if !rusk_abi::verify_bls(digest, unstake.public_key, unstake.signature)
        {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to withdraw a note from this contract
        // containing the value of the stake
        let transfer_module = rusk_abi::transfer_module();
        let _: bool = self
            .transact(
                transfer_module,
                "wfct",
                &Wfct {
                    value,
                    note: unstake.note,
                    proof: unstake.proof,
                },
            )
            .expect("Withdrawing note from contract should be successful");
    }

    pub fn withdraw(self: &mut State<Self>, withdraw: Withdraw) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

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

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(loaded_stake);

        // verify signature
        let digest = withdraw_signature_message(
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
        let transfer_module = rusk_abi::transfer_module();
        let _: bool = self
            .transact(
                transfer_module,
                "mint",
                &Mint {
                    address: withdraw.address,
                    value: reward,
                    nonce: withdraw.nonce,
                },
            )
            .expect("Minting a reward note should succeed");
    }

    pub fn allow(&mut self, allow: Allow) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        if self.is_allowlisted(&allow.public_key) {
            panic!("Address already allowed!");
        }

        if !self.is_owner(&allow.owner) {
            panic!("Can only be called by a contract owner!");
        }

        // increment the signature counter
        let owner_stake = self.load_stake_mut(&allow.owner);

        let owner_counter = owner_stake.counter();
        owner_stake.increment_counter();

        drop(owner_stake);

        // verify signature
        let digest =
            allow_signature_message(owner_counter, allow.public_key).to_vec();

        if !rusk_abi::verify_bls(digest, allow.owner, allow.signature) {
            panic!("Invalid signature!");
        }

        self.insert_allowlist(allow.public_key);
    }

    /// Gets a reference to a stake.
    pub fn get_stake(&self, key: &PublicKey) -> Option<&StakeDataWrapper> {
        self.stakes.get(&key.to_bytes())
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &PublicKey,
    ) -> Option<&mut StakeDataWrapper> {
        self.stakes.get_mut(&key.to_bytes())
    }

    /// Pushes the given `stake` onto the state for a given `public_key`.
    pub fn insert_stake(&mut self, public_key: PublicKey, stake: StakeData) {
        self.stakes
            .insert(public_key.to_bytes(), StakeDataWrapper(stake));
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_stake_mut(
        &mut self,
        pk: &PublicKey,
    ) -> &mut StakeDataWrapper {
        let is_missing = self.stakes.get(&pk.to_bytes()).is_none();

        if is_missing {
            let stake = StakeDataWrapper(StakeData {
                amount: None,
                reward: 0,
                counter: 0,
            });
            self.stakes.insert(pk.to_bytes(), stake);
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        self.stakes.get_mut(&pk.to_bytes()).unwrap()
    }

    /// Rewards a `public_key` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(&mut self, public_key: &PublicKey, value: u64) {
        let stake = self.load_stake_mut(public_key);
        stake.increase_reward(value);
    }

    /// Gets a vector of all public keys and stakes.
    pub fn stakes(&self) -> Vec<(PublicKey, StakeData)> {
        self.stakes
            .iter()
            .map(|(k, v)| (PublicKey::from_bytes(k).unwrap(), v.clone().0))
            .collect()
    }

    /// Gets a vector of all allowlisted keys.
    pub fn stakers_allowlist(&self) -> Vec<PublicKey> {
        self.allowlist
            .iter()
            .map(|e| PublicKey::from_bytes(e).unwrap())
            .collect()
    }

    /// Gets a vector of all owner keys.
    pub fn owners(&self) -> Vec<PublicKey> {
        self.owners
            .iter()
            .map(|e| PublicKey::from_bytes(e).unwrap())
            .collect()
    }

    pub fn add_owner(&mut self, owner: PublicKey) {
        if !self.is_owner(&owner) {
            self.owners.insert(owner.to_bytes());
        }
    }

    pub fn is_owner(&self, owner: &PublicKey) -> bool {
        self.owners.get(&owner.to_bytes()).is_some()
    }

    pub fn insert_allowlist(&mut self, staker: PublicKey) {
        if !self.is_allowlisted(&staker) {
            self.allowlist.insert(staker.to_bytes());
        }
    }

    pub fn is_allowlisted(&self, staker: &PublicKey) -> bool {
        self.allowlist.get(&staker.to_bytes()).is_some()
    }
}
