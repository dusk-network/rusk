// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's stake contract.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use rkyv::{Archive, Deserialize, Serialize};

use crate::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    Signature as BlsSignature,
};
use crate::transfer::withdraw::Withdraw as TransferWithdraw;
use crate::{dusk, ContractId, Dusk};

/// ID of the genesis stake contract
pub const STAKE_CONTRACT: ContractId = crate::reserved(0x2);

/// Epoch used for stake operations
pub const EPOCH: u64 = 2160;

/// Number of warnings before being penalized
pub const STAKE_WARNINGS: u8 = 1;

/// Calculate the block height at which the next epoch takes effect.
#[must_use]
pub const fn next_epoch(block_height: u64) -> u64 {
    let to_next_epoch = EPOCH - (block_height % EPOCH);
    block_height + to_next_epoch
}

/// Stake a value on the stake contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Stake {
    chain_id: u8,
    keys: StakeKeys,
    value: u64,
    signature: DoubleSignature,
}

impl Stake {
    const MESSAGE_SIZE: usize =
        1 + BlsPublicKey::SIZE + BlsPublicKey::SIZE + u64::SIZE;

    /// Create a new stake.
    #[must_use]
    pub fn new(sk: &BlsSecretKey, value: u64, chain_id: u8) -> Self {
        let key = BlsPublicKey::from(sk);

        let keys = StakeKeys {
            account: key,
            funds: key,
        };

        let mut stake = Stake {
            chain_id,
            keys,
            value,
            signature: DoubleSignature::default(),
        };

        let msg = stake.signature_message();

        stake.signature = DoubleSignature {
            account: sk.sign(&msg),
            funds: sk.sign(&msg),
        };

        stake
    }

    /// Account to which the stake will belong.
    #[must_use]
    pub fn keys(&self) -> &StakeKeys {
        &self.keys
    }

    /// Account to which the stake will belong.
    #[must_use]
    pub fn account(&self) -> &BlsPublicKey {
        &self.keys.account
    }

    /// Value to stake.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Returns the chain ID of the stake.
    #[must_use]
    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }

    /// Signature of the stake.
    #[must_use]
    pub fn signature(&self) -> &DoubleSignature {
        &self.signature
    }

    /// Return the message that is used as the input to the signature.
    #[must_use]
    pub fn signature_message(&self) -> [u8; Self::MESSAGE_SIZE] {
        let mut bytes = [0u8; Self::MESSAGE_SIZE];

        bytes[0] = self.chain_id;
        let mut offset = 1;

        bytes[offset..offset + BlsPublicKey::SIZE]
            .copy_from_slice(&self.keys.account.to_bytes());
        offset += BlsPublicKey::SIZE;

        bytes[offset..offset + BlsPublicKey::SIZE]
            .copy_from_slice(&self.keys.funds.to_bytes());
        offset += BlsPublicKey::SIZE;

        bytes[offset..offset + u64::SIZE]
            .copy_from_slice(&self.value.to_bytes());

        bytes
    }
}

/// Withdraw some value from the stake contract.
///
/// This is used in both `unstake` and `withdraw`.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    account: BlsPublicKey,
    withdraw: TransferWithdraw,
    signature: DoubleSignature,
}

impl Withdraw {
    /// Create a new withdraw call.
    #[must_use]
    pub fn new(sk: &BlsSecretKey, withdraw: TransferWithdraw) -> Self {
        let account = BlsPublicKey::from(sk);
        let mut stake_withdraw = Withdraw {
            account,
            withdraw,
            signature: DoubleSignature::default(),
        };

        let msg = stake_withdraw.signature_message();

        stake_withdraw.signature = DoubleSignature {
            account: sk.sign(&msg),
            funds: sk.sign(&msg),
        };

        stake_withdraw
    }

    /// The public key to withdraw from.
    #[must_use]
    pub fn account(&self) -> &BlsPublicKey {
        &self.account
    }

    /// The inner withdrawal to pass to the transfer contract.
    #[must_use]
    pub fn transfer_withdraw(&self) -> &TransferWithdraw {
        &self.withdraw
    }

    /// Signature of the withdraw.
    #[must_use]
    pub fn signature(&self) -> &DoubleSignature {
        &self.signature
    }

    /// Signature message used for [`Withdraw`].
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.account.to_bytes());
        bytes.extend(self.withdraw.wrapped_signature_message());

        bytes
    }
}

/// Event emitted after a stake contract operation is performed.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeEvent {
    /// Keys associated to the event.
    pub keys: StakeKeys,
    /// Effective value of the relevant operation, be it `stake`,
    /// `unstake`,`withdraw`
    pub value: u64,
    /// The locked amount involved in the operation (e.g., for `stake` or
    /// `unstake`). Defaults to zero for operations that do not involve
    /// locking.
    pub locked: u64,
}

impl StakeEvent {
    /// Creates a new `StakeEvent` with the specified keys and value.
    ///
    /// ### Parameters
    /// - `keys`: The keys associated with the stake event.
    /// - `value`: The effective value of the operation (e.g., `stake`,
    ///   `unstake`, `withdraw`).
    ///
    /// The `locked` amount is initialized to zero by default.
    #[must_use]
    pub fn new(keys: StakeKeys, value: u64) -> Self {
        Self {
            keys,
            value,
            locked: 0,
        }
    }
    /// Sets the locked amount for the `StakeEvent`.
    ///
    /// ### Parameters
    /// - `locked`: The locked amount associated with the operation.
    #[must_use]
    pub fn locked(mut self, locked: u64) -> Self {
        self.locked = locked;
        self
    }
}

/// Event emitted after a slash operation is performed.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SlashEvent {
    /// Account slashed.
    pub account: BlsPublicKey,
    /// Slashed amount
    pub value: u64,
    /// New eligibility for the slashed account
    pub next_eligibility: u64,
}

/// The minimum amount of Dusk one can stake.
pub const MINIMUM_STAKE: Dusk = dusk(1_000.0);

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
    Debug, Default, Clone, Copy, PartialEq, Eq, Archive, Deserialize, Serialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeData {
    /// Amount staked and eligibility.
    pub amount: Option<StakeAmount>,
    /// The reward for participating in consensus.
    pub reward: u64,
    /// Faults
    pub faults: u8,
    /// Hard Faults
    pub hard_faults: u8,
}

/// Keys that identify a stake
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Archive, Deserialize, Serialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeKeys {
    /// Key used for the consensus
    pub account: BlsPublicKey,
    /// Key used for handle funds (stake/unstake/withdraw)
    pub funds: BlsPublicKey,
}

/// Signature used to handle funds (stake/unstake/withdraw)
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Archive, Deserialize, Serialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct DoubleSignature {
    /// Signature created with the account key
    pub account: BlsSignature,
    /// Signature created with the funds key
    pub funds: BlsSignature,
}

impl StakeData {
    /// An empty stake.
    pub const EMPTY: Self = Self {
        amount: None,
        reward: 0,
        faults: 0,
        hard_faults: 0,
    };

    /// Create a new stake given its initial `value` and `reward`, together with
    /// the `block_height` of its creation.
    #[must_use]
    pub const fn new(value: u64, reward: u64, block_height: u64) -> Self {
        let eligibility = Self::eligibility_from_height(block_height);
        Self::with_eligibility(value, reward, eligibility)
    }

    /// Create a new stake given its initial `value` and `reward`, together with
    /// the `eligibility`.
    #[must_use]
    pub const fn with_eligibility(
        value: u64,
        reward: u64,
        eligibility: u64,
    ) -> Self {
        let amount = match value {
            0 => None,
            _ => Some(StakeAmount {
                value,
                eligibility,
                locked: 0,
            }),
        };

        Self {
            amount,
            reward,
            faults: 0,
            hard_faults: 0,
        }
    }

    /// Returns true if the stake is valid - meaning there is an `amount` staked
    /// and the given `block_height` is larger or equal to the stake's
    /// eligibility. If there is no `amount` staked this is false.
    #[must_use]
    pub fn is_valid(&self, block_height: u64) -> bool {
        match &self.amount {
            Some(amount) => block_height >= amount.eligibility,
            None => false,
        }
    }

    /// Compute the eligibility of a stake from the starting block height.
    #[must_use]
    pub const fn eligibility_from_height(block_height: u64) -> u64 {
        StakeAmount::eligibility_from_height(block_height)
    }

    /// Check if there is no amount left to withdraw
    ///
    /// Return true if both stake and rewards are 0
    pub fn is_empty(&self) -> bool {
        let stake = self
            .amount
            .as_ref()
            .map(StakeAmount::total_funds)
            .unwrap_or_default();
        self.reward + stake == 0
    }
}

/// Value staked and eligibility.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Archive, Deserialize, Serialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeAmount {
    /// The value staked.
    pub value: u64,
    /// The amount that has been locked (due to a soft slash).
    pub locked: u64,
    /// The eligibility of the stake.
    pub eligibility: u64,
}

impl StakeAmount {
    /// Create a new stake amount.
    #[must_use]
    pub const fn new(value: u64, block_height: u64) -> Self {
        let eligibility = Self::eligibility_from_height(block_height);
        Self::with_eligibility(value, eligibility)
    }

    /// Create a new stake given its initial `value` and `reward`, together with
    /// the `eligibility`.
    #[must_use]
    pub const fn with_eligibility(value: u64, eligibility: u64) -> Self {
        Self {
            value,
            eligibility,
            locked: 0,
        }
    }

    /// Compute the eligibility of a stake from the starting block height.
    #[must_use]
    pub const fn eligibility_from_height(block_height: u64) -> u64 {
        let maturity_blocks = EPOCH;
        next_epoch(block_height) + maturity_blocks
    }

    /// Move `amount` to locked value
    pub fn lock_amount(&mut self, amount: u64) {
        self.value -= amount;
        self.locked += amount;
    }

    /// Get the total funds belonging to the stake (value + locked)
    #[must_use]
    pub fn total_funds(&self) -> u64 {
        self.value + self.locked
    }
}

/// Used in a `reward` call to reward a given account with an amount of Dusk,
/// and emitted as an event, once a reward succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Reward {
    /// The account to be rewarded.
    pub account: BlsPublicKey,
    /// The amount to reward.
    pub value: u64,
    /// The reason for the reward.
    pub reason: RewardReason,
}

/// The reason that a reward is issued.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum RewardReason {
    /// The fixed amount awarded to a generator.
    GeneratorFixed,
    /// Extra amount awarded to a generator.
    GeneratorExtra,
    /// Amount awarded to a voter.
    Voter,
    /// Amount awarded for another reason, such as rewarding Dusk.
    Other,
}
