// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;
use tonic::Status;
use canonical::CanonError;

/// Errors returned by this crate
#[derive(Debug)]
pub enum CliError {
    CorruptedFile,
    KeyNotFound,
    KeyAlreadyExists,
    InvalidPhrase,

    Network(tonic::transport::Error),
    Connection(tonic::Status),

    Bytes(dusk_bytes::Error),
    Canon(CanonError),
    IO(io::Error),
}

impl From<dusk_bytes::Error> for CliError {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<CanonError> for CliError{
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}

impl From<io::Error> for CliError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<Status> for CliError {
    fn from(s: Status) -> Self {
        Self::Connection(s)
    }
}
