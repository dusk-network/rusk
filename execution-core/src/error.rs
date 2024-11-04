// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error-type for execution-core.

use alloc::string::String;
use core::fmt;

/// The execution-core error type.
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
    /// Dusk-bytes InvalidData error
    InvalidData,
    /// Dusk-bytes BadLength error
    BadLength(usize, usize),
    /// Dusk-bytes InvalidChar error
    InvalidChar(char, usize),
    /// Rkyv serialization.
    Rkyv(String),
    /// The provided memo is too large. Contains the memo size used. The max
    /// size is [`MAX_MEMO_SIZE`].
    ///
    /// [`MAX_MEMO_SIZE`]: crate::transfer::data::MAX_MEMO_SIZE
    MemoTooLarge(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Execution-Core Error: {:?}", &self)
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
