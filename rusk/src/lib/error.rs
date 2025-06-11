// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{fmt, io};

use dusk_bytes::Serializable;
use dusk_core::{
    signatures::bls::PublicKey as BlsPublicKey, transfer::phoenix::CoreError,
    BlsScalar, Error as ExecErr,
};
use dusk_vm::Error as VMError;

#[derive(Debug)]
pub enum Error {
    /// Failed to register a backend for persistence
    BackendRegistrationFailed,
    /// Failed to restore a network state from disk
    RestoreFailed,
    /// Proof verification failure
    ProofVerification,
    /// Out of gas in block execution
    OutOfGas,
    /// Repeated nullifier in transaction verification
    RepeatingNullifiers(Vec<BlsScalar>),
    /// Repeated nullifier in the same transaction
    DoubleNullifiers,
    /// Repeating a nonce that has already been used
    RepeatingNonce(Box<BlsPublicKey>, u64),
    /// Wrong inputs and/or outputs in the transaction verification
    InvalidCircuitArguments(usize, usize),
    /// Failed to build a Rusk instance
    BuilderInvalidState,
    /// Failed to fetch opening
    OpeningPositionNotFound(u64),
    /// Failed to fetch opening due to undefined Note
    OpeningNoteUndefined(u64),
    /// Bytes Serialization Errors
    Serialization(dusk_bytes::Error),
    /// Originating from transaction-creation
    Transaction(ExecErr),
    /// Originating from Phoenix.
    Phoenix(CoreError),
    /// Piecrust VM internal Errors
    Vm(VMError),
    /// IO Errors
    Io(io::Error),
    /// Other
    Other(Box<dyn std::error::Error>),
    /// Commit not found amongst existing commits
    CommitNotFound([u8; 32]),
    /// Invalid credits count
    InvalidCreditsCount(u64, usize),
    /// Memo too large
    MemoTooLarge(usize),
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::Other(err)
    }
}

impl From<VMError> for Error {
    fn from(err: VMError) -> Self {
        Error::Vm(err)
    }
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
        }
    }
}

impl From<dusk_bytes::Error> for Error {
    fn from(err: dusk_bytes::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<CoreError> for Error {
    fn from(pe: CoreError) -> Self {
        Self::Phoenix(pe)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BackendRegistrationFailed => {
                write!(f, "Failed to register a backend for persistence")
            }
            Error::RestoreFailed => {
                write!(f, "Failed to restore a network state")
            }
            Error::BuilderInvalidState => {
                write!(f, "Failed to build a Rusk instance")
            }
            Error::OpeningPositionNotFound(pos) => {
                write!(f, "Failed to fetch opening of position {pos}")
            }
            Error::OpeningNoteUndefined(pos) => {
                write!(f, "Note {pos} not found, opening of position")
            }
            Error::Serialization(err) => {
                write!(f, "Serialization Error: {err:?}")
            }
            Error::Vm(err) => write!(f, "VM Error: {err}"),
            Error::Io(err) => write!(f, "IO Error: {err}"),
            Error::Transaction(err) => write!(f, "Transaction Error: {err}"),
            Error::Phoenix(err) => write!(f, "Phoenix error: {err}"),
            Error::Other(err) => write!(f, "Other error: {err}"),
            Error::ProofVerification => write!(f, "Proof verification failure"),
            Error::OutOfGas => write!(f, "Out of gas"),
            Error::RepeatingNullifiers(n) => {
                write!(f, "Nullifiers already spent: {n:?}")
            }
            Error::DoubleNullifiers => write!(f, "Double nullifiers"),
            Error::RepeatingNonce(account, nonce) => {
                let encoded_account =
                    bs58::encode(&account.to_bytes()).into_string();
                write!(f, "Nonce repeat: {encoded_account} {nonce}")
            }
            Error::InvalidCircuitArguments(inputs_len, outputs_len) => {
                write!(f,"Expected: 0 < (inputs: {inputs_len}) < 5, 0 ≤ (outputs: {outputs_len}) < 3")
            }
            Error::CommitNotFound(commit_id) => {
                write!(f, "Commit not found, id = {}", hex::encode(commit_id),)
            }
            Error::InvalidCreditsCount(height, credits) => {
                write!(f, "Invalid credits: H= {height}, credits= {credits}",)
            }
            Error::MemoTooLarge(size) => {
                write!(f, "The memo size {size} is too large")
            }
        }
    }
}
