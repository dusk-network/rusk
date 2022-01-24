// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_pki::PublicKey;
use std::{fmt, io};
use tonic::Status;

#[derive(Debug)]
pub enum Error {
    /// Failed to register a backend for persistence
    BackendRegistrationFailed,
    /// Failed to restore a network state from disk
    RestoreFailed,
    /// Failed to fetch opening
    OpeningPositionNotFound(u64),
    /// Failed to fetch opening due to undefined Note
    OpeningNoteUndefined(u64),
    /// Bytes Serialization Errors
    Serialization(dusk_bytes::Error),
    /// Rusk VM internal Errors
    Vm(rusk_vm::VMError),
    /// IO Errors
    Io(io::Error),
    /// Persistence Errors
    Persistence(microkelvin::PersistError),
    /// Transfer Contract Errors
    TransferContract(transfer_contract::Error),
    /// Tonic Status Errors
    Status(tonic::Status),
    /// Canonical Errors
    Canonical(canonical::CanonError),
    /// Stake not found for key.
    StakeNotFound(PublicKey),
}

impl std::error::Error for Error {}

impl From<rusk_vm::VMError> for Error {
    fn from(err: rusk_vm::VMError) -> Self {
        Error::Vm(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<microkelvin::PersistError> for Error {
    fn from(err: microkelvin::PersistError) -> Self {
        Error::Persistence(err)
    }
}

impl From<transfer_contract::Error> for Error {
    fn from(err: transfer_contract::Error) -> Self {
        Error::TransferContract(err)
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BackendRegistrationFailed => {
                write!(f, "Failed to register a backend for persistence")
            }
            Error::RestoreFailed => {
                write!(f, "Failed to restore a network state")
            }
            Error::OpeningPositionNotFound(pos) => {
                write!(f, "Failed to fetch opening of position {}", pos)
            }
            Error::OpeningNoteUndefined(pos) => {
                write!(f, "Note {} not found, opening of position", pos)
            }
            Error::Serialization(err) => {
                write!(f, "Serialization Error: {:?}", err)
            }
            Error::Vm(err) => write!(f, "VM Error: {}", err),
            Error::Io(err) => write!(f, "IO Error: {}", err),
            Error::Persistence(err) => {
                write!(f, "Persistence Error: {:?}", err)
            }
            Error::TransferContract(err) => {
                write!(f, "Transfer Contract Error: {}", err)
            }
            Error::Status(err) => write!(f, "Status Error: {}", err),
            Error::Canonical(err) => write!(f, "Canonical Error: {:?}", err),
            Error::StakeNotFound(pk) => {
                write!(f, "Couldn't find stake for {:?}", pk.to_bytes())
            }
        }
    }
}

impl From<Error> for Status {
    fn from(err: Error) -> Self {
        Status::internal(format!("{}", err))
    }
}
