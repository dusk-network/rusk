// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's stake contract.

extern crate alloc;

use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    BlockHeight, BlsScalar, StakePublicKey, StakeSignature, StealthAddress,
};

/// Epoch used for stake operations
pub const EPOCH: u64 = 2160;

/// Calculate the block height at which the next epoch takes effect.
#[must_use]
pub const fn next_epoch(block_height: BlockHeight) -> u64 {
    let to_next_epoch = EPOCH - (block_height % EPOCH);
    block_height + to_next_epoch
}

/// Stake a value on the stake contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Stake {
    /// Public key to which the stake will belong.
    pub public_key: StakePublicKey,
    /// Signature belonging to the given public key.
    pub signature: StakeSignature,
    /// Value to stake.
    pub value: u64,
}

impl Stake {
    const MESSAGE_SIZE: usize = u64::SIZE + u64::SIZE;
    /// Return the digest to be signed in the `stake` function of the stake
    /// contract.
    #[must_use]
    pub fn signature_message(
        counter: u64,
        value: u64,
    ) -> [u8; Self::MESSAGE_SIZE] {
        let mut bytes = [0u8; Self::MESSAGE_SIZE];

        bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
        bytes[u64::SIZE..].copy_from_slice(&value.to_bytes());

        bytes
    }
}

/// Unstake a value from the stake contract.
#[derive(Debug, Clone, PartialEq, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Unstake {
    /// Public key to unstake.
    pub public_key: StakePublicKey,
    /// Signature belonging to the given public key.
    pub signature: StakeSignature,
    /// The address to mint to.
    pub address: StealthAddress,
}

impl Unstake {
    const MESSAGE_SIZE: usize = u64::SIZE + u64::SIZE + StealthAddress::SIZE;
    /// Signature message used for [`Unstake`].
    #[must_use]
    pub fn signature_message(
        counter: u64,
        value: u64,
        address: StealthAddress,
    ) -> [u8; Self::MESSAGE_SIZE] {
        let mut bytes = [0u8; Self::MESSAGE_SIZE];

        bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
        bytes[u64::SIZE..u64::SIZE + u64::SIZE]
            .copy_from_slice(&value.to_bytes());
        bytes[u64::SIZE + u64::SIZE
            ..u64::SIZE + u64::SIZE + StealthAddress::SIZE]
            .copy_from_slice(&address.to_bytes());

        bytes
    }
}

/// Withdraw the accumulated reward.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    /// Public key to withdraw the rewards.
    pub public_key: StakePublicKey,
    /// Signature belonging to the given public key.
    pub signature: StakeSignature,
    /// The address to mint to.
    pub address: StealthAddress,
    /// A nonce to prevent replay.
    pub nonce: BlsScalar,
}

impl Withdraw {
    const MESSAGE_SIZE: usize =
        u64::SIZE + StealthAddress::SIZE + BlsScalar::SIZE;

    /// Signature message used for [`Withdraw`].
    #[must_use]
    pub fn signature_message(
        counter: u64,
        address: StealthAddress,
        nonce: BlsScalar,
    ) -> [u8; Self::MESSAGE_SIZE] {
        let mut bytes = [0u8; Self::MESSAGE_SIZE];

        bytes[..u64::SIZE].copy_from_slice(&counter.to_bytes());
        bytes[u64::SIZE..u64::SIZE + StealthAddress::SIZE]
            .copy_from_slice(&address.to_bytes());
        bytes[u64::SIZE + StealthAddress::SIZE..]
            .copy_from_slice(&nonce.to_bytes());

        bytes
    }
}

/// Event emitted after a stake contract operation is performed.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct StakingEvent {
    /// Public key which is relevant to the event.
    pub public_key: StakePublicKey,
    /// Value of the relevant operation, be it stake, unstake, withdrawal,
    /// reward, or slash.
    pub value: u64,
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
