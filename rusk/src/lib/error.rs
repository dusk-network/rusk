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
    Vm(piecrust::Error),
    /// IO Errors
    Io(io::Error),
    /// Address Parsing error
    AddrParsing(std::net::AddrParseError),
    /// Persistence Errors
    Persistence(microkelvin::PersistError),
    /// Stake Contract Errors
    StakeContract(stake_contract::Error),
    /// Transfer Contract Errors
    TransferContract(transfer_contract::Error),
    /// Tonic Status Errors
    Status(tonic::Status),
    /// Tonic client/server error
    Transport(tonic::transport::Error),
    /// Canonical Errors
    Canonical(canonical::CanonError),
    /// Tonic Status Errors
    Status(tonic::Status),
    /// Bad block height in coinbase (got, expected)
    CoinbaseBlockHeight(u64, u64),
    /// Bad dusk spent in coinbase (got, expected).
    CoinbaseDuskSpent(Dusk, Dusk),
    /// Other
    Other(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::Other(err)
    }
}

impl From<piecrust::Error> for Error {
    fn from(err: piecrust::Error) -> Self {
        Error::Vm(err)
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

impl From<tonic::Status> for Error {
    fn from(err: tonic::Status) -> Self {
        Error::Status(err)
    }
}

impl From<canonical::CanonError> for Error {
    fn from(err: canonical::CanonError) -> Self {
        Error::Canonical(err)
    }
}

impl From<tonic::transport::Error> for Error {
    fn from(err: tonic::transport::Error) -> Self {
        Error::Transport(err)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(err: std::net::AddrParseError) -> Self {
        Error::AddrParsing(err)
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
                write!(f, "Serialization Error: {:?}", err)
            }
            Error::Vm(err) => write!(f, "VM Error: {}", err),
            Error::Io(err) => write!(f, "IO Error: {}", err),
            Error::AddrParsing(err) => {
                write!(f, "Address Parsing Error: {}", err)
            }
            Error::Persistence(err) => {
                write!(f, "Persistence Error: {:?}", err)
            }
            Error::StakeContract(err) => {
                write!(f, "Stake Contract Error: {}", err)
            }
            Error::TransferContract(err) => {
                write!(f, "Transfer Contract Error: {}", err)
            }
            Error::Status(err) => write!(f, "Status Error: {}", err),
            Error::Transport(err) => write!(f, "Transport Error: {}", err),
            Error::Canonical(err) => write!(f, "Canonical Error: {:?}", err),
            Error::Phoenix(err) => write!(f, "Phoenix error: {}", err),
            Error::Other(err) => write!(f, "Other error: {}", err),
                write!(f, "Serialization Error: {err:?}")
            }
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
        }
    }
}

impl From<Error> for tonic::Status {
    fn from(err: Error) -> Self {
        tonic::Status::internal(format!("{err}"))
    }
}
