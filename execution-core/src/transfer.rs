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

use crate::{
    signatures::bls::{
        PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
    },
    transfer::withdraw::{Withdraw, WithdrawReceiver},
    BlsScalar, ContractError, ContractId, Error,
};

pub mod data;
pub mod moonlight;
pub mod phoenix;
pub mod withdraw;

/// ID of the genesis transfer contract
pub const TRANSFER_CONTRACT: ContractId = crate::reserved(0x1);

/// Panic of "Nonce not ready to be used yet"
pub const PANIC_NONCE_NOT_READY: &str = "Nonce not ready to be used yet";

use data::{ContractCall, ContractDeploy, TransactionData};
use moonlight::Transaction as MoonlightTransaction;
use phoenix::{
    Note, Prove, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    Sender, StealthAddress, Transaction as PhoenixTransaction,
    NOTES_TREE_DEPTH,
};

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
        change_pk: &PhoenixPublicKey,
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
    ) -> Result<Self, Error> {
        Ok(Self::Phoenix(PhoenixTransaction::new::<R, P>(
            rng,
            sender_sk,
            change_pk,
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
        )?))
    }

    /// Create a new moonlight transaction.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - the memo, if given, is too large
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight(
        from_sk: &AccountSecretKey,
        to_account: Option<AccountPublicKey>,
        value: u64,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
    ) -> Result<Self, Error> {
        Ok(Self::Moonlight(MoonlightTransaction::new(
            from_sk, to_account, value, deposit, gas_limit, gas_price, nonce,
            chain_id, data,
        )?))
    }

    /// Return the sender of the account for Moonlight transactions.
    #[must_use]
    pub fn from_account(&self) -> Option<&AccountPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => Some(tx.from_account()),
        }
    }

    /// Return the receiver of the transaction for Moonlight transactions, if it
    /// exists.
    #[must_use]
    pub fn to_account(&self) -> Option<&AccountPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => tx.to_account(),
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

    /// Return the stealth address for returning funds for Phoenix transactions.
    #[must_use]
    pub fn stealth_address(&self) -> Option<&StealthAddress> {
        match self {
            Self::Phoenix(tx) => Some(tx.stealth_address()),
            Self::Moonlight(_) => None,
        }
    }

    /// Returns the sender data for Phoenix transactions.
    #[must_use]
    pub fn sender(&self) -> Option<&Sender> {
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

/// The payload sent by a contract to the transfer contract to transfer some of
/// its funds to another contract.
#[derive(Debug, Clone, Archive, PartialEq, Eq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferToContract {
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
pub struct TransferToAccount {
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
    pub contract: ContractId,
    /// The value withdrawn.
    pub value: u64,
    /// The receiver of the value.
    pub receiver: WithdrawReceiver,
}

impl From<Withdraw> for WithdrawEvent {
    fn from(w: Withdraw) -> Self {
        Self {
            contract: w.contract,
            value: w.value,
            receiver: w.receiver,
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
    /// The value converted.
    pub value: u64,
    /// The receiver of the value.
    pub receiver: WithdrawReceiver,
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
            value: withdraw.value,
            receiver: withdraw.receiver,
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
    /// The value deposited.
    pub value: u64,
    /// The receiver of the value.
    pub receiver: ContractId,
}

/// Event data emitted on a transfer from a contract to a contract.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferToContractEvent {
    /// The sender of the funds.
    pub sender: ContractId,
    /// The value transferred.
    pub value: u64,
    /// The receiver of the funds.
    pub receiver: ContractId,
}

/// Event data emitted on a transfer from a contract to a Moonlight account.
#[derive(Debug, Clone, Archive, PartialEq, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferToAccountEvent {
    /// The sender of the funds.
    pub sender: ContractId,
    /// The value transferred.
    pub value: u64,
    /// The receiver of the funds.
    pub receiver: AccountPublicKey,
}

/// Event data emitted on a phoenix transaction's completion.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct PhoenixTransactionEvent {
    /// Nullifiers of the notes spent during the transaction.
    pub nullifiers: Vec<BlsScalar>,
    /// Notes produced during the transaction.
    pub notes: Vec<Note>,
    /// Output of the transaction.
    pub output: TransactionOutput,
    /// Gas spent by the transaction.
    pub gas_spent: u64,
}

/// Event data emitted on a moonlight transaction's completion.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct MoonlightTransactionEvent {
    /// The account that initiated the transaction.
    pub from: AccountPublicKey,
    /// The receiver of the funds if any were transferred.
    pub to: Option<AccountPublicKey>,
    /// Transfer amount
    pub value: u64,
    /// Output of the transaction.
    pub output: TransactionOutput,
    /// Gas spent by the transaction.
    pub gas_spent: u64,
}

/// The output of a transaction.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum TransactionOutput {
    /// The result of a contract call. The `Ok()` result omits the output of
    /// the contract, since it may be too large to emit.
    Call(Result<(), ContractError>),
    /// The ID of the contract deployed.
    Deploy(ContractId),
    /// The memo used in the transaction.
    Memo(Vec<u8>),
    /// There is no output to the transaction. This means that there was no
    /// contract call, deployment, or memo included in the transaction.
    None,
}
