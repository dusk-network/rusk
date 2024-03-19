// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{fmt, io};

use dusk_bls12_381::BlsScalar;
use rusk_abi::dusk::Dusk;

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
    /// Originating from Phoenix.
    Phoenix(phoenix_core::Error),
    /// Piecrust VM internal Errors
    Vm(rusk_abi::Error),
    /// IO Errors
    Io(io::Error),
    /// Bad block height in coinbase (got, expected)
    CoinbaseBlockHeight(u64, u64),
    /// Bad dusk spent in coinbase (got, expected).
    CoinbaseDuskSpent(Dusk, Dusk),
    /// Proof creation error
    #[cfg(feature = "prover")]
    ProofCreation(rusk_prover::ProverError),
    /// Failed to produce proper state
    #[cfg(feature = "node")]
    InconsistentState(dusk_consensus::operations::VerificationOutput),
    /// Other
    Other(Box<dyn std::error::Error>),
    /// Commit not found amongst existing commits
    CommitNotFound([u8; 32]),
    /// Last epoch not found when expected
    LastEpochNotFound,
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::Other(err)
    }
}

impl From<rusk_abi::Error> for Error {
    fn from(err: rusk_abi::Error) -> Self {
        Error::Vm(err)
    }
}

#[cfg(feature = "prover")]
impl From<rusk_prover::ProverError> for Error {
    fn from(err: rusk_prover::ProverError) -> Self {
        Error::ProofCreation(err)
    }
}

impl From<dusk_bytes::Error> for Error {
    fn from(err: dusk_bytes::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<phoenix_core::Error> for Error {
    fn from(pe: phoenix_core::Error) -> Self {
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
            Error::Phoenix(err) => write!(f, "Phoenix error: {err}"),
            Error::Other(err) => write!(f, "Other error: {err}"),
            Error::CoinbaseBlockHeight(got, expected) => write!(
                f,
                "Coinbase has block height {got}, expected {expected}"
            ),
            Error::CoinbaseDuskSpent(got, expected) => {
                write!(f, "Coinbase has dusk spent {got}, expected {expected}")
            }
            Error::ProofVerification => write!(f, "Proof verification failure"),
            Error::OutOfGas => write!(f, "Out of gas"),
            Error::RepeatingNullifiers(n) => {
                write!(f, "Nullifiers repeat: {n:?}")
            }
            Error::InvalidCircuitArguments(inputs_len, outputs_len) => {
                write!(f,"Expected: 0 < (inputs: {inputs_len}) < 5, 0 ≤ (outputs: {outputs_len}) < 3")
            }
            #[cfg(feature = "prover")]
            Error::ProofCreation(e) => {
                write!(f, "Proof creation error: {e}")
            }
            #[cfg(feature = "node")]
            Error::InconsistentState(verification_output) => {
                write!(
                    f,
                    "Inconsistent state verification data {verification_output}",
                )
            }
            Error::CommitNotFound(commit_id) => {
                write!(f, "Commit not found, id = {}", hex::encode(commit_id),)
            }
            Error::LastEpochNotFound => {
                write!(f, "Last epoch not found when expected")
            }
        }
    }
}
