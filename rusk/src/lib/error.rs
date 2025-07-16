// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::transfer::phoenix::CoreError as PhoenixError;
use dusk_core::{BlsScalar, Error as ExecErr};
use dusk_vm::Error as VMError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Out of gas in block execution
    #[error("Out of gas")]
    OutOfGas,
    /// Repeated nullifier in transaction verification
    #[error("Nullifiers already spent: {0:?}")]
    RepeatingNullifiers(Vec<BlsScalar>),
    /// Repeated nullifier in the same transaction
    #[error("Double nullifiers")]
    DoubleNullifiers,
    /// Repeating a nonce that has already been used
    #[error("Nonce repeat: {} {1}", bs58::encode(.0.to_bytes()).into_string())]
    RepeatingNonce(Box<BlsPublicKey>, u64),
    /// Wrong inputs and/or outputs in the transaction verification
    #[error("Expected: 0 < (inputs: {0}) < 5, 0 ≤ (outputs: {1}) < 3")]
    InvalidCircuitArguments(usize, usize),
    /// Failed to fetch opening
    #[error("Failed to fetch opening of position {0}")]
    OpeningPositionNotFound(u64),
    /// Bytes Serialization Errors
    #[error("Serialization Error: {0:?}")]
    Serialization(dusk_bytes::Error),
    /// Originating from transaction-creation
    #[error("Transaction Error: {0}")]
    Transaction(ExecErr),
    /// Originating from Phoenix.
    #[error("Phoenix Error: {0}")]
    Phoenix(PhoenixError),
    /// Piecrust VM internal Errors
    #[error("VM Error: {0}")]
    Vm(#[from] VMError),
    /// IO Errors
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    /// Other
    #[error("Other Error: {0}")]
    Other(#[from] Box<dyn std::error::Error>),
    /// Commit not found amongst existing commits
    #[error("Commit not found, id = {}", hex::encode(.0))]
    CommitNotFound([u8; 32]),
    /// Invalid credits count
    #[error("Invalid credits: H= {0}, credits= {1}")]
    InvalidCreditsCount(u64, usize),
    /// Memo too large
    #[error("The memo size {0} is too large")]
    MemoTooLarge(usize),
    /// Blob related errors
    #[error("Blob error: {0}")]
    Blob(String),
}

impl From<dusk_core::Error> for Error {
    fn from(err: ExecErr) -> Self {
        match err {
            ExecErr::InsufficientBalance => {
                Self::Transaction(ExecErr::InsufficientBalance)
            }
            ExecErr::Replay => Self::Transaction(ExecErr::Replay),
            ExecErr::PhoenixOwnership => {
                Self::Transaction(ExecErr::PhoenixOwnership)
            }
            ExecErr::PhoenixCircuit(e) => {
                Self::Transaction(ExecErr::PhoenixCircuit(e))
            }
            ExecErr::PhoenixProver(e) => {
                Self::Transaction(ExecErr::PhoenixProver(e))
            }
            ExecErr::InvalidData => {
                Self::Serialization(dusk_bytes::Error::InvalidData)
            }
            ExecErr::BadLength(found, expected) => {
                Self::Serialization(dusk_bytes::Error::BadLength {
                    found,
                    expected,
                })
            }
            ExecErr::InvalidChar(ch, index) => {
                Self::Serialization(dusk_bytes::Error::InvalidChar {
                    ch,
                    index,
                })
            }
            ExecErr::Rkyv(e) => Self::Transaction(ExecErr::Rkyv(e)),
            ExecErr::MemoTooLarge(size) => Self::MemoTooLarge(size),
            ExecErr::Blob(e) => Self::Blob(e),
        }
    }
}

impl From<dusk_bytes::Error> for Error {
    fn from(err: dusk_bytes::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<PhoenixError> for Error {
    fn from(pe: PhoenixError) -> Self {
        Self::Phoenix(pe)
    }
}
