// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to Dusk's transfer contract that are shared across the
//! network.

use alloc::vec::Vec;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError};
use phoenix_core::{Note, StealthAddress};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    transfer::{ContractCall, ContractDeploy},
    BlsPublicKey, BlsScalar, BlsSignature, Sender,
};

mod moonlight;
mod phoenix;

pub use moonlight::{
    Payload as MoonlightPayload, Transaction as MoonlightTransaction,
};
pub use phoenix::{
    Payload as PhoenixPayload, Transaction as PhoenixTransaction,
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
    #[must_use]
    pub fn phoenix(payload: PhoenixPayload, proof: impl Into<Vec<u8>>) -> Self {
        Self::Phoenix(PhoenixTransaction::new(payload, proof))
    }

    /// Create a new moonlight transaction.
    #[must_use]
    pub fn moonlight(
        payload: MoonlightPayload,
        signature: BlsSignature,
    ) -> Self {
        Self::Moonlight(MoonlightTransaction::new(payload, signature))
    }

    /// Return the sender of the account for Moonlight transactions.
    #[must_use]
    pub fn from(&self) -> Option<&BlsPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => Some(&tx.payload.from),
        }
    }

    /// Return the receiver of the transaction for Moonlight transactions, if it
    /// exists.
    #[must_use]
    pub fn to(&self) -> Option<&BlsPublicKey> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => tx.payload.to.as_ref(),
        }
    }

    /// Return the value transferred in a Moonlight transaction.
    #[must_use]
    pub fn value(&self) -> Option<u64> {
        match self {
            Self::Phoenix(_) => None,
            Self::Moonlight(tx) => Some(tx.payload.value),
        }
    }

    /// Returns the nullifiers of the transaction, if the transaction is a
    /// moonlight transaction, the result will be empty.
    #[must_use]
    pub fn nullifiers(&self) -> &[BlsScalar] {
        match self {
            Self::Phoenix(tx) => &tx.payload.tx_skeleton.nullifiers,
            Self::Moonlight(_) => &[],
        }
    }

    /// Return the root of the UTXO tree for Phoenix transactions.
    #[must_use]
    pub fn root(&self) -> Option<&BlsScalar> {
        match self {
            Self::Phoenix(tx) => Some(&tx.payload.tx_skeleton.root),
            Self::Moonlight(_) => None,
        }
    }

    /// Return the UTXO outputs of the transaction.
    #[must_use]
    pub fn outputs(&self) -> &[Note] {
        match self {
            Self::Phoenix(tx) => &tx.payload.tx_skeleton.outputs,
            Self::Moonlight(_) => &[],
        }
    }

    /// Return the stealth address for returning funds for Phoenix transactions.
    #[must_use]
    pub fn stealth_address(&self) -> Option<&StealthAddress> {
        match self {
            Self::Phoenix(tx) => Some(&tx.payload.fee.stealth_address),
            Self::Moonlight(_) => None,
        }
    }

    /// Returns the sender data for Phoenix transactions.
    #[must_use]
    pub fn sender(&self) -> Option<&Sender> {
        match self {
            Self::Phoenix(tx) => Some(&tx.payload.fee.sender),
            Self::Moonlight(_) => None,
        }
    }

    /// Returns the deposit of the transaction.
    #[must_use]
    pub fn deposit(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.payload.tx_skeleton.deposit,
            Self::Moonlight(tx) => tx.payload.deposit,
        }
    }

    /// Returns the gas limit of the transaction.
    #[must_use]
    pub fn gas_limit(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.payload.fee.gas_limit,
            Self::Moonlight(tx) => tx.payload.gas_limit,
        }
    }

    /// Returns the gas price of the transaction.
    #[must_use]
    pub fn gas_price(&self) -> u64 {
        match self {
            Self::Phoenix(tx) => tx.payload.fee.gas_price,
            Self::Moonlight(tx) => tx.payload.gas_price,
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

    /// Return input bytes to hash the payload.
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
