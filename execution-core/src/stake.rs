// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's stake contract.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    signatures::bls::{
        Error as BlsError, MultisigPublicKey as BlsMultisigPublicKey,
        MultisigSignature as BlsMultisigSignature, PublicKey as BlsPublicKey,
        SecretKey as BlsSecretKey,
    },
    transfer::withdraw::Withdraw as TransferWithdraw,
    ContractId,
};

use crate::{dusk, Dusk};

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
    nonce: u64,
    signature: BlsMultisigSignature,
}

impl Stake {
    const MESSAGE_SIZE: usize =
        1 + BlsPublicKey::SIZE + BlsPublicKey::SIZE + u64::SIZE + u64::SIZE;

    /// Create a new stake.
    #[must_use]
    pub fn new(
        sk: &BlsSecretKey,
        value: u64,
        nonce: u64,
        chain_id: u8,
    ) -> Self {
        let key = BlsPublicKey::from(sk);

        let mut stake = Stake {
            chain_id,
            keys: StakeKeys {
                account: key,
                funds: key,
            },
            value,
            nonce,
            signature: BlsMultisigSignature::default(),
        };

        let msg = stake.signature_message();

        let first_sig = sk.sign_multisig(&key, &msg);
        stake.signature = first_sig.aggregate(&[first_sig]);

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

    /// Nonce used for replay protection. Nonces are strictly increasing and
    /// incremental, meaning that for a transaction to be valid, only the
    /// current nonce + 1 can be used.
    ///
    /// The current nonce is queryable via the stake contract in the form of
    /// [`StakeData`].
    #[must_use]
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Returns the chain ID of the stake.
    #[must_use]
    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }

    /// Signature of the stake.
    #[must_use]
    pub fn signature(&self) -> &BlsMultisigSignature {
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
        offset += u64::SIZE;

        bytes[offset..offset + u64::SIZE]
            .copy_from_slice(&self.nonce.to_bytes());

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
    signature: BlsMultisigSignature,
}

impl Withdraw {
    /// Create a new withdraw call.
    #[must_use]
    pub fn new(sk: &BlsSecretKey, withdraw: TransferWithdraw) -> Self {
        let account = BlsPublicKey::from(sk);
        let mut stake_withdraw = Withdraw {
            account,
            withdraw,
            signature: BlsMultisigSignature::default(),
        };

        let msg = stake_withdraw.signature_message();

        let first_sig = sk.sign_multisig(&account, &msg);
        stake_withdraw.signature = first_sig.aggregate(&[first_sig]);

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
    pub fn signature(&self) -> &BlsMultisigSignature {
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
    /// Value of the relevant operation, be it `stake`, `unstake`,`withdraw`
    pub value: u64,
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
    /// Nonce used for replay protection. Nonces are strictly increasing and
    /// incremental, meaning that for a transaction to be valid, only the
    /// current nonce + 1 can be used.
    ///
    /// The current nonce is queryable via the stake contract in the form of
    /// [`StakeData`].
    pub nonce: u64,
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

impl StakeKeys {
    /// Return the `MultisigPublicKey` associated to this `StakeKeys`
    ///
    /// # Errors
    ///
    /// Look at `MultisigPublicKey::aggregate`
    pub fn multisig_pk(&self) -> Result<BlsMultisigPublicKey, BlsError> {
        BlsMultisigPublicKey::aggregate(&[self.account, self.funds])
    }
}

impl StakeData {
    /// An empty stake.
    pub const EMPTY: Self = Self {
        amount: None,
        reward: 0,
        nonce: 0,
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
            nonce: 0,
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
}

const STAKE_DATA_SIZE: usize =
    u8::SIZE + StakeAmount::SIZE + u64::SIZE + u64::SIZE + u8::SIZE + u8::SIZE;

impl Serializable<STAKE_DATA_SIZE> for StakeData {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut buf = &buf[..];

        // if the tag is zero we skip the bytes
        let tag = u8::from_reader(&mut buf)?;
        let amount = match tag {
            0 => {
                buf = &buf[..StakeAmount::SIZE];
                None
            }
            _ => Some(StakeAmount::from_reader(&mut buf)?),
        };

        let reward = u64::from_reader(&mut buf)?;
        let nonce = u64::from_reader(&mut buf)?;

        let faults = u8::from_reader(&mut buf)?;
        let hard_faults = u8::from_reader(&mut buf)?;

        Ok(Self {
            amount,
            reward,
            nonce,
            faults,
            hard_faults,
        })
    }

    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        const ZERO_AMOUNT: [u8; StakeAmount::SIZE] = [0u8; StakeAmount::SIZE];

        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        match &self.amount {
            None => {
                writer.write(&0u8.to_bytes());
                writer.write(&ZERO_AMOUNT);
            }
            Some(amount) => {
                writer.write(&1u8.to_bytes());
                writer.write(&amount.to_bytes());
            }
        }

        writer.write(&self.reward.to_bytes());
        writer.write(&self.nonce.to_bytes());

        writer.write(&self.faults.to_bytes());
        writer.write(&self.hard_faults.to_bytes());

        buf
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
}

const STAKE_AMOUNT_SIZE: usize = u64::SIZE + u64::SIZE + u64::SIZE;

impl Serializable<STAKE_AMOUNT_SIZE> for StakeAmount {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut buf = &buf[..];

        let value = u64::from_reader(&mut buf)?;
        let locked = u64::from_reader(&mut buf)?;
        let eligibility = u64::from_reader(&mut buf)?;

        Ok(Self {
            value,
            locked,
            eligibility,
        })
    }

    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        writer.write(&self.value.to_bytes());
        writer.write(&self.locked.to_bytes());
        writer.write(&self.eligibility.to_bytes());

        buf
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
