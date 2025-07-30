// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error-type for dusk-core.

use alloc::string::{String, ToString};
use core::fmt;

/// The dusk-core error type.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// There is not sufficient balance to cover the transaction costs.
    InsufficientBalance,
    /// A transaction input has been used already.
    Replay,
    /// The input-note doesn't belong to the given key.
    PhoenixOwnership,
    /// The transaction circuit wasn't found or is incorrect.
    PhoenixCircuit(String),
    /// The transaction circuit prover wasn't found or couldn't be created.
    PhoenixProver(String),
    /// Dusk-bytes `InvalidData` error
    InvalidData,
    /// Dusk-bytes `BadLength` error
    BadLength(usize, usize),
    /// Dusk-bytes `InvalidChar` error
    InvalidChar(char, usize),
    /// Rkyv serialization.
    Rkyv(String),
    /// Blob KZG related.
    Blob(String),
    /// The provided memo is too large. Contains the memo size used. The max
    /// size is [`MAX_MEMO_SIZE`].
    ///
    /// [`MAX_MEMO_SIZE`]: crate::transfer::data::MAX_MEMO_SIZE
    MemoTooLarge(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dusk-Core Error: {:?}", &self)
    }
}

impl From<phoenix_core::Error> for Error {
    fn from(core_error: phoenix_core::Error) -> Self {
        #[allow(clippy::match_same_arms)]
        match core_error {
            phoenix_core::Error::InvalidNoteType(_) => Self::InvalidData,
            phoenix_core::Error::MissingViewKey => Self::PhoenixOwnership,
            phoenix_core::Error::InvalidEncryption => Self::PhoenixOwnership,
            phoenix_core::Error::InvalidData => Self::InvalidData,
            phoenix_core::Error::BadLength(found, expected) => {
                Self::BadLength(found, expected)
            }
            phoenix_core::Error::InvalidChar(ch, index) => {
                Self::InvalidChar(ch, index)
            }
        }
    }
}

impl From<dusk_bytes::Error> for Error {
    fn from(bytes_error: dusk_bytes::Error) -> Self {
        match bytes_error {
            dusk_bytes::Error::InvalidData => Self::InvalidData,
            dusk_bytes::Error::BadLength { found, expected } => {
                Self::BadLength(found, expected)
            }
            dusk_bytes::Error::InvalidChar { ch, index } => {
                Self::InvalidChar(ch, index)
            }
        }
    }
}

/// Error type for checking transaction conditions.
///
/// This error is used to indicate that a transaction does not meet the
/// minimum requirements for deployment or blob gas charges.
#[derive(Debug, Clone, PartialEq)]
pub enum TxPreconditionError {
    /// The gas price is too low to deploy a transaction.
    DeployLowPrice(u64),
    /// The gas limit is too low to deploy a transaction.
    DeployLowLimit(u64),
    /// The gas limit is too low to cover the blob gas charges.
    BlobLowLimit(u64),
    /// No blob attached to the transaction.
    BlobEmpty,
    /// Too many blobs attached to the transaction.
    BlobTooMany(usize),
}

impl TxPreconditionError {
    /// Return the implementation of toString to be used inside the VM.
    ///
    /// Replacing this with the standard display will break the state root
    /// backward compatibility
    #[must_use]
    pub fn legacy_to_string(&self) -> String {
        match self {
            TxPreconditionError::DeployLowPrice(_) => {
                "gas price too low to deploy"
            }
            TxPreconditionError::DeployLowLimit(_) => {
                "not enough gas to deploy"
            }
            TxPreconditionError::BlobLowLimit(_) => "not enough gas for blobs",
            TxPreconditionError::BlobEmpty => {
                "no blob attached to the transaction"
            }
            TxPreconditionError::BlobTooMany(_) => {
                "too many blobs in the transaction"
            }
        }
        .to_string()
    }
}
