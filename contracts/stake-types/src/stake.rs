// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Block height type alias
pub type BlockHeight = u64;

/// Epoch used for stake operations
pub const EPOCH: u64 = 2160;

/// Calculate the block height at which the next epoch takes effect.
#[must_use]
pub const fn next_epoch(block_height: BlockHeight) -> u64 {
    let to_next_epoch = EPOCH - (block_height % EPOCH);
    block_height + to_next_epoch
}

/// The representation of a public key's stake.
///
/// A user can stake for a particular `amount` larger in value than the
/// `MINIMUM_STAKE` value and is `reward`ed for participating in the consensus.
/// A stake is valid only after a particular block height - called the
/// eligibility.
///
/// To keep track of the number of interactions a public key has had with the
/// contract a `counter` is used to prevent repeat attacks - where the same
/// signature could be used to prove ownership of the secret key in two
/// different transactions.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, Archive, Deserialize, Serialize,
)]
#[archive_attr(derive(CheckBytes))]
#[allow(clippy::module_name_repetitions)]
pub struct StakeData {
    /// Amount staked and eligibility.
    pub amount: Option<(u64, BlockHeight)>,
    /// The reward for participating in consensus.
    pub reward: u64,
    /// The signature counter to prevent replay.
    pub counter: u64,
    /// How many times this provisioners has been shifted
    pub shift_count: u64,
}

impl StakeData {
    /// Create a new stake given its initial `value` and `reward`, together with
    /// the `block_height` of its creation.
    #[must_use]
    pub const fn new(
        value: u64,
        reward: u64,
        block_height: BlockHeight,
    ) -> Self {
        let eligibility = Self::eligibility_from_height(block_height);
        Self::with_eligibility(value, reward, eligibility)
    }

    /// Create a new stake given its initial `value` and `reward`, together with
    /// the `eligibility`.
    #[must_use]
    pub const fn with_eligibility(
        value: u64,
        reward: u64,
        eligibility: BlockHeight,
    ) -> Self {
        let amount = match value {
            0 => None,
            _ => Some((value, eligibility)),
        };

        Self {
            amount,
            reward,
            counter: 0,
            shift_count: 0
        }
    }

    /// Returns the value the user is staking, together with its eligibility.
    #[must_use]
    pub const fn amount(&self) -> Option<&(u64, BlockHeight)> {
        self.amount.as_ref()
    }

    /// Returns the value of the reward.
    #[must_use]
    pub const fn reward(&self) -> u64 {
        self.reward
    }

    /// Returns the interaction count of the stake.
    #[must_use]
    pub const fn counter(&self) -> u64 {
        self.counter
    }

    /// Insert a stake [`amount`] with a particular `value`, starting from a
    /// particular `block_height`.
    ///
    /// # Panics
    /// If the value is zero or the stake already contains an amount.
    pub fn insert_amount(&mut self, value: u64, block_height: BlockHeight) {
        assert_ne!(value, 0, "A stake can't have zero value");
        assert!(self.amount.is_none(), "Can't stake twice for the same key!");

        let eligibility = Self::eligibility_from_height(block_height);
        self.amount = Some((value, eligibility));
    }

    /// Increases the held reward by the given `value`.
    pub fn increase_reward(&mut self, value: u64) {
        self.reward += value;
    }

    /// Removes the total [`amount`] staked.
    ///
    /// # Panics
    /// If the stake has no amount.
    pub fn remove_amount(&mut self) -> (u64, BlockHeight) {
        self.amount
            .take()
            .expect("Can't withdraw non-existing amount!")
    }

    /// Sets the reward to zero.
    pub fn deplete_reward(&mut self) {
        self.reward = 0;
    }

    /// Increment the interaction [`counter`].
    pub fn increment_counter(&mut self) {
        self.counter += 1;
    }

    /// Returns true if the stake is valid - meaning there is an amount staked
    /// and the given `block_height` is larger or equal to the stake's
    /// eligibility. If there is no `amount` staked this is false.
    #[must_use]
    pub fn is_valid(&self, block_height: BlockHeight) -> bool {
        self.amount
            .map(|(_, eligibility)| block_height >= eligibility)
            .unwrap_or_default()
    }

    /// Compute the eligibility of a stake from the starting block height.
    #[must_use]
    pub const fn eligibility_from_height(block_height: BlockHeight) -> u64 {
        let maturity_blocks = EPOCH;
        next_epoch(block_height) + maturity_blocks
    }
}
