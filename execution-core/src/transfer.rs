// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to Dusk's transfer contract that are shared across the
//! network.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError};
use poseidon_merkle::Opening;
use rand::{CryptoRng, RngCore};
use rkyv::{Archive, Deserialize, Serialize};

use crate::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use crate::{BlsScalar, ContractId, Error};

use data::{ContractCall, ContractDeploy, TransactionData};
use moonlight::Transaction as MoonlightTransaction;
use phoenix::{
    Note, Prove, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    Sender, StealthAddress, Transaction as PhoenixTransaction,
    NOTES_TREE_DEPTH,
};
use withdraw::{Withdraw, WithdrawReceiver};

#[cfg(feature = "serde")]
use crate::serde_support;
#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

pub mod data;
pub mod moonlight;
pub mod phoenix;
pub mod withdraw;

/// ID of the genesis transfer contract
pub const TRANSFER_CONTRACT: ContractId = crate::reserved(0x1);

/// Panic of "Nonce not ready to be used yet"
pub const PANIC_NONCE_NOT_READY: &str = "Nonce not ready to be used yet";

/// Topic for the moonlight transaction event.
pub const MOONLIGHT_TOPIC: &str = "moonlight";
/// Topic for the phoenix transaction event.
pub const PHOENIX_TOPIC: &str = "phoenix";
/// Topic for the contract to contract transaction event.
pub const CONTRACT_TO_CONTRACT_TOPIC: &str = "contract_to_contract";
/// Topic for the contract to account transaction event.
pub const CONTRACT_TO_ACCOUNT_TOPIC: &str = "contract_to_account";
/// Topic for the withdraw event.
pub const WITHDRAW_TOPIC: &str = "withdraw";
/// Topic for the deposit event.
pub const DEPOSIT_TOPIC: &str = "deposit";
/// Topic for the convert event.
pub const CONVERT_TOPIC: &str = "convert";
/// Topic for the mint event.
pub const MINT_TOPIC: &str = "mint";

/// The transaction used by the transfer contract.
#[derive(Debug, Clone, Archive, PartialEq, Eq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[allow(clippy::large_enum_variant)]
pub enum Transaction {
    /// A phoenix transaction.
    Phoenix(PhoenixTransaction),
    /// A moonlight transaction.
    Moonlight(MoonlightTransaction),
}

impl Transaction {
    /// Create a new phoenix transaction.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - one of the input-notes doesn't belong to the `sender_sk`
    /// - the transaction input doesn't cover the transaction costs
    /// - the `inputs` vector is either empty or larger than 4 elements
    /// - the `inputs` vector contains duplicate `Note`s
    /// - the `Prove` trait is implemented incorrectly
    #[allow(clippy::too_many_arguments)]
    pub fn phoenix<R: RngCore + CryptoRng, P: Prove>(
        rng: &mut R,
        sender_sk: &PhoenixSecretKey,
        refund_pk: &PhoenixPublicKey,
        receiver_pk: &PhoenixPublicKey,
        inputs: Vec<(Note, Opening<(), NOTES_TREE_DEPTH>)>,
        root: BlsScalar,
        transfer_value: u64,
        obfuscated_transaction: bool,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
        prover: &P,
    ) -> Result<Self, Error> {
        Ok(Self::Phoenix(PhoenixTransaction::new::<R, P>(
            rng,
            sender_sk,
            refund_pk,
            receiver_pk,
            inputs,
            root,
            transfer_value,
            obfuscated_transaction,
            deposit,
            gas_limit,
            gas_price,
            chain_id,
            data,
            prover,
        )?))
    }

    /// Create a new moonlight transaction.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - the memo, if given, is too large
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight(
        sender_sk: &AccountSecretKey,
        receiver: Option<AccountPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Self, Error> {
        Ok(Self::Moonlight(MoonlightTransaction::new(
            sender_sk, receiver, value, deposit, gas_limit, gas_price, nonce,
            chain_id, data,
        )?))
    }

    /// Return the sender of the account for Moonlight transactions.
    #[must_use]
    pub fn moonlight_sender(&self) -> Option<&AccountPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => Some(tx.sender()),
        }
    }

    /// Return the receiver of the transaction for Moonlight transactions, if it
    /// exists.
    #[must_use]
    pub fn moonlight_receiver(&self) -> Option<&AccountPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => tx.receiver(),
        }
    }

    /// Return the value transferred in a Moonlight transaction.
    #[must_use]
    pub fn value(&self) -> Option<u64> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => Some(tx.value()),
        }
    }

    /// Returns the nullifiers of the transaction, if the transaction is a
    /// moonlight transaction, the result will be empty.
    #[must_use]
    pub fn nullifiers(&self) -> &[BlsScalar] {
        match self {
            Self::Phoenix(tx) => tx.nullifiers(),
            Self::Moonlight(_) => &[],
        }
    }

    /// Return the root of the UTXO tree for Phoenix transactions.
    #[must_use]
    pub fn root(&self) -> Option<&BlsScalar> {
        match self {
            Self::Phoenix(tx) => Some(tx.root()),
            Self::Moonlight(_) => None,
        }
    }

    /// Return the UTXO outputs of the transaction.
    #[must_use]
    pub fn outputs(&self) -> &[Note] {
        match self {
            Self::Phoenix(tx) => &tx.outputs()[..],
            Self::Moonlight(_) => &[],
        }
    }

    /// Returns the sender data for Phoenix transactions.
    #[must_use]
    pub fn phoenix_sender(&self) -> Option<&Sender> {
        match self {
            Self::Phoenix(tx) => Some(tx.sender()),
            Self::Moonlight(_) => None,
        }
    }

    /// Returns the deposit of the transaction.
    #[must_use]
    pub fn deposit(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.deposit(),
            Self::Moonlight(tx) => tx.deposit(),
        }
    }

    /// Returns the gas limit of the transaction.
    #[must_use]
    pub fn gas_limit(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.gas_limit(),
            Self::Moonlight(tx) => tx.gas_limit(),
        }
    }

    /// Returns the gas price of the transaction.
    #[must_use]
    pub fn gas_price(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.gas_price(),
            Self::Moonlight(tx) => tx.gas_price(),
        }
    }

    /// Returns the refund-address of the transaction.
    #[must_use]
    pub fn refund_address(&self) -> RefundAddress {
        match self {
            Self::Phoenix(tx) => RefundAddress::Phoenix(tx.stealth_address()),
            Self::Moonlight(tx) => {
                RefundAddress::Moonlight(tx.refund_address())
            }
        }
    }

    /// Return the contract call data, if there is any.
    #[must_use]
    pub fn call(&self) -> Option<&ContractCall> {
        match self {
            Self::Phoenix(tx) => tx.call(),
            Self::Moonlight(tx) => tx.call(),
        }
    }

    /// Return the contract deploy data, if there is any.
    #[must_use]
    pub fn deploy(&self) -> Option<&ContractDeploy> {
        match self {
            Self::Phoenix(tx) => tx.deploy(),
            Self::Moonlight(tx) => tx.deploy(),
        }
    }

    /// Returns the memo used with the transaction, if any.
    #[must_use]
    pub fn memo(&self) -> Option<&[u8]> {
        match self {
            Self::Phoenix(tx) => tx.memo(),
            Self::Moonlight(tx) => tx.memo(),
        }
    }

    /// Creates a modified clone of this transaction if it contains data for
    /// deployment, clones all fields except for the bytecode' 'bytes' part.
    /// Returns none if the transaction is not a deployment transaction.
    #[must_use]
    pub fn strip_off_bytecode(&self) -> Option<Self> {
        Some(match self {
            Transaction::Phoenix(tx) => {
                Transaction::Phoenix(tx.strip_off_bytecode()?)
            }
            Transaction::Moonlight(tx) => {
                Transaction::Moonlight(tx.strip_off_bytecode()?)
            }
        })
    }

    /// Serialize the transaction into a byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        match self {
            Self::Phoenix(tx) => {
                bytes.push(0);
                bytes.extend(tx.to_var_bytes());
            }
            Self::Moonlight(tx) => {
                bytes.push(1);
                bytes.extend(tx.to_var_bytes());
            }
        }

        bytes
    }

    /// Deserialize the transaction from a byte slice.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        Ok(match u8::from_reader(&mut buf)? {
            0 => Self::Phoenix(PhoenixTransaction::from_slice(buf)?),
            1 => Self::Moonlight(MoonlightTransaction::from_slice(buf)?),
            _ => return Err(BytesError::InvalidData),
        })
    }

    /// Return input bytes to hash the transaction.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the transaction again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        match self {
            Self::Phoenix(tx) => tx.to_hash_input_bytes(),
            Self::Moonlight(tx) => tx.to_hash_input_bytes(),
        }
    }

    /// Create the unique transaction hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        match self {
            Self::Phoenix(tx) => tx.hash(),
            Self::Moonlight(tx) => tx.hash(),
        }
    }
}

impl From<PhoenixTransaction> for Transaction {
    fn from(tx: PhoenixTransaction) -> Self {
        Self::Phoenix(tx)
    }
}

impl From<MoonlightTransaction> for Transaction {
    fn from(tx: MoonlightTransaction) -> Self {
        Self::Moonlight(tx)
    }
}

/// Enum defining the address to refund unspent gas to for both Phoenix and
/// Moonlight transactions.
pub enum RefundAddress<'a> {
    /// The address of the Phoenix refund note.
    Phoenix(&'a StealthAddress),
    /// The moonlight account to which to send the refund.
    Moonlight(&'a AccountPublicKey),
}

/// The payload sent by a contract to the transfer contract to transfer some of
/// its funds to another contract.
#[derive(Debug, Clone, Archive, PartialEq, Eq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractToContract {
    /// Contract to transfer funds to.
    pub contract: ContractId,
    /// Amount to send to the contract.
    pub value: u64,
    /// Function name to call on the contract.
    pub fn_name: String,
    /// Extra data sent along with [`ReceiveFromContract`]
    pub data: Vec<u8>,
}

/// The payload sent by the transfer contract to a contract receiving funds from
/// another contract.
#[derive(Debug, Clone, Archive, PartialEq, Eq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ReceiveFromContract {
    /// Contract that sent the funds.
    pub contract: ContractId,
    /// Amount sent by the contract.
    pub value: u64,
    /// Extra data sent by the sender.
    pub data: Vec<u8>,
}

/// The payload sent by a contract to the transfer contract to transfer some of
/// its funds to an account.
#[derive(Debug, Clone, Archive, PartialEq, Eq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractToAccount {
    /// Account to transfer funds to.
    pub account: AccountPublicKey,
    /// Amount to send to the account.
    pub value: u64,
}

/// Event data emitted on a withdrawal from a contract.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct WithdrawEvent {
    /// The contract withdrawn from.
    pub sender: ContractId,
    /// The receiver of the value.
    pub receiver: WithdrawReceiver,
    /// The value withdrawn.
    pub value: u64,
}

impl From<Withdraw> for WithdrawEvent {
    fn from(w: Withdraw) -> Self {
        Self {
            sender: w.contract,
            receiver: w.receiver,
            value: w.value,
        }
    }
}

/// Event data emitted on a conversion from Phoenix to Moonlight, and
/// vice-versa.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ConvertEvent {
    /// The originator of the conversion, if it is possible to determine. From
    /// Moonlight to Phoenix it is possible, but the same cannot be done the
    /// other way round.
    pub sender: Option<AccountPublicKey>,
    /// The receiver of the value.
    pub receiver: WithdrawReceiver,
    /// The value converted.
    pub value: u64,
}

impl ConvertEvent {
    /// Convert a sender and a withdraw into a conversion event.
    #[must_use]
    pub fn from_withdraw_and_sender(
        sender: Option<AccountPublicKey>,
        withdraw: &Withdraw,
    ) -> Self {
        Self {
            sender,
            receiver: withdraw.receiver,
            value: withdraw.value,
        }
    }
}

/// Event data emitted on a deposit to a contract.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct DepositEvent {
    /// The originator of the deposit, if it is possible to determine. If the
    /// depositor is using Moonlight this will be available. If they're using
    /// Phoenix it will not.
    pub sender: Option<AccountPublicKey>,
    /// The receiver of the value.
    pub receiver: ContractId,
    /// The value deposited.
    pub value: u64,
}

/// Event data emitted on a transfer from a contract to a contract.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractToContractEvent {
    /// The sender of the funds.
    pub sender: ContractId,
    /// The receiver of the funds.
    pub receiver: ContractId,
    /// The value transferred.
    pub value: u64,
}

/// Event data emitted on a transfer from a contract to a Moonlight account.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractToAccountEvent {
    /// The sender of the funds.
    pub sender: ContractId,
    /// The receiver of the funds.
    pub receiver: AccountPublicKey,
    /// The value transferred.
    pub value: u64,
}

/// Event data emitted on a phoenix transaction's completion.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct PhoenixTransactionEvent {
    /// Nullifiers of the notes spent during the transaction.
    pub nullifiers: Vec<BlsScalar>,
    /// Notes produced during the transaction.
    pub notes: Vec<Note>,
    /// The memo included in the transaction.
    pub memo: Vec<u8>,
    /// Gas spent by the transaction.
    pub gas_spent: u64,
    /// Optional gas-refund note if the refund is positive.
    pub refund_note: Option<Note>,
}

/// Event data emitted on a moonlight transaction's completion.
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct MoonlightTransactionEvent {
    /// The account that initiated the transaction.
    #[cfg_attr(feature = "serde", serde(with = "serde_support::pubk"))]
    pub sender: AccountPublicKey,
    /// The receiver of the funds if any were transferred.
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_support::optional_pubk")
    )]
    pub receiver: Option<AccountPublicKey>,
    /// Transfer amount
    pub value: u64,
    /// The memo included in the transaction.
    #[cfg_attr(feature = "serde", serde(with = "serde_support::hex_serde"))]
    pub memo: Vec<u8>,
    /// Gas spent by the transaction.
    pub gas_spent: u64,
    /// Optional refund-info in the case that the refund-address is different
    /// from the sender.
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_support::pubk_u64_tuple")
    )]
    pub refund_info: Option<(AccountPublicKey, u64)>,
}
