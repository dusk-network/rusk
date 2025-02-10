// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to withdrawing funds into moonlight of phoenix Dusk.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use rand::{CryptoRng, RngCore};
use rkyv::{Archive, Deserialize, Serialize};

use crate::abi::ContractId;
use crate::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
    Signature as AccountSignature,
};
use crate::signatures::schnorr::{
    SecretKey as NoteSecretKey, Signature as NoteSignature,
};
use crate::transfer::phoenix::StealthAddress;
use crate::BlsScalar;

/// Withdrawal information, proving the intent of a user to withdraw from a
/// contract.
///
/// This structure is meant to be passed to a contract by a caller. The contract
/// is then responsible for calling `withdraw` in the transfer contract to
/// settle it, if it wants to allow the withdrawal.
///
/// e.g. the stake contract uses it as a call argument for the `unstake`
/// function
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Withdraw {
    pub(super) contract: ContractId,
    pub(super) value: u64,
    pub(super) receiver: WithdrawReceiver,
    token: WithdrawReplayToken,
    signature: WithdrawSignature,
}

impl Withdraw {
    /// Create a new contract withdrawal.
    ///
    /// # Panics
    /// When the receiver does not match the secret key passed.
    #[must_use]
    pub fn new<'a, R: RngCore + CryptoRng>(
        rng: &mut R,
        sk: impl Into<WithdrawSecretKey<'a>>,
        contract: ContractId,
        value: u64,
        receiver: WithdrawReceiver,
        token: WithdrawReplayToken,
    ) -> Self {
        let mut withdraw = Self {
            contract,
            value,
            receiver,
            token,
            signature: WithdrawSignature::Moonlight(AccountSignature::default()),
        };

        let sk = sk.into();

        match (&sk, &receiver) {
            (WithdrawSecretKey::Phoenix(_), WithdrawReceiver::Moonlight(_)) => {
                panic!("Moonlight receiver with phoenix signer");
            }
            (WithdrawSecretKey::Moonlight(_), WithdrawReceiver::Phoenix(_)) => {
                panic!("Phoenix receiver with moonlight signer");
            }
            _ => {}
        }

        let msg = withdraw.signature_message();

        match sk {
            WithdrawSecretKey::Phoenix(sk) => {
                let digest = BlsScalar::hash_to_scalar(&msg);
                let signature = sk.sign(rng, digest);
                withdraw.signature = signature.into();
            }
            WithdrawSecretKey::Moonlight(sk) => {
                let signature = sk.sign(&msg);
                withdraw.signature = signature.into();
            }
        }

        withdraw
    }

    /// The contract to withraw from.
    #[must_use]
    pub fn contract(&self) -> &ContractId {
        &self.contract
    }

    /// The amount to withdraw.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.value
    }

    /// The receiver of the value.
    #[must_use]
    pub fn receiver(&self) -> &WithdrawReceiver {
        &self.receiver
    }

    /// The unique token to prevent replay.
    #[must_use]
    pub fn token(&self) -> &WithdrawReplayToken {
        &self.token
    }

    /// Signature of the withdrawal.
    #[must_use]
    pub fn signature(&self) -> &WithdrawSignature {
        &self.signature
    }

    /// Return the message that is used as the input to the signature.
    #[must_use]
    pub fn signature_message(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend(self.contract.as_bytes());
        bytes.extend(self.value.to_bytes());

        match self.receiver {
            WithdrawReceiver::Phoenix(address) => {
                bytes.extend(address.to_bytes());
            }
            WithdrawReceiver::Moonlight(account) => {
                bytes.extend(account.to_bytes());
            }
        }

        match &self.token {
            WithdrawReplayToken::Phoenix(nullifiers) => {
                for n in nullifiers {
                    bytes.extend(n.to_bytes());
                }
            }
            WithdrawReplayToken::Moonlight(nonce) => {
                bytes.extend(nonce.to_bytes());
            }
        }

        bytes
    }

    /// Returns the message that should be "mixed in" as input for a signature
    /// of an item that wraps a [`Withdraw`].
    ///
    /// One example of this is [`crate::stake::Withdraw`].
    #[must_use]
    pub fn wrapped_signature_message(&self) -> Vec<u8> {
        let mut bytes = self.signature_message();
        bytes.extend(self.signature.to_var_bytes());
        bytes
    }
}

/// The receiver of the [`Withdraw`] value.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawReceiver {
    /// The stealth address to withdraw to, when the withdrawal is into Phoenix
    /// notes.
    Phoenix(StealthAddress),
    /// The account to withdraw to, when the withdrawal is to a Moonlight
    /// account.
    Moonlight(AccountPublicKey),
}

/// The token used for replay protection in a [`Withdraw`]. This is the same as
/// the encapsulating transaction's fields.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawReplayToken {
    /// The nullifiers of the encapsulating Phoenix transaction, when the
    /// transaction is paid for using Phoenix notes.
    Phoenix(Vec<BlsScalar>),
    /// The nonce of the encapsulating Moonlight transaction, when the
    /// transaction is paid for using a Moonlight account.
    Moonlight(u64),
}

/// The secret key used for signing a [`Withdraw`].
///
/// When the withdrawal is into Phoenix notes, a [`NoteSecretKey`] should be
/// used. When the withdrawal is into a Moonlight account an
/// [`AccountSecretKey`] should be used.
#[derive(Debug, Clone, PartialEq)]
pub enum WithdrawSecretKey<'a> {
    /// The secret key used to sign a withdrawal into Phoenix notes.
    Phoenix(&'a NoteSecretKey),
    /// The secret key used to sign a withdrawal into a Moonlight account.
    Moonlight(&'a AccountSecretKey),
}

impl<'a> From<&'a NoteSecretKey> for WithdrawSecretKey<'a> {
    fn from(sk: &'a NoteSecretKey) -> Self {
        Self::Phoenix(sk)
    }
}

impl<'a> From<&'a AccountSecretKey> for WithdrawSecretKey<'a> {
    fn from(sk: &'a AccountSecretKey) -> Self {
        Self::Moonlight(sk)
    }
}

/// The signature used for a [`Withdraw`].
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum WithdrawSignature {
    /// A transaction withdrawing to Phoenix must sign using their
    /// [`NoteSecretKey`] which can only be generated by the note-owner's
    /// [`crate::transfer::phoenix::SecretKey`].
    Phoenix(NoteSignature),
    /// A transaction withdrawing to Moonlight - must sign using their
    /// [`AccountSecretKey`].
    Moonlight(AccountSignature),
}

impl WithdrawSignature {
    fn to_var_bytes(&self) -> Vec<u8> {
        match self {
            WithdrawSignature::Phoenix(sig) => sig.to_bytes().to_vec(),
            WithdrawSignature::Moonlight(sig) => sig.to_bytes().to_vec(),
        }
    }
}

impl From<NoteSignature> for WithdrawSignature {
    fn from(sig: NoteSignature) -> Self {
        Self::Phoenix(sig)
    }
}

impl From<AccountSignature> for WithdrawSignature {
    fn from(sig: AccountSignature) -> Self {
        Self::Moonlight(sig)
    }
}
