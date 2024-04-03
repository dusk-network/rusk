// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::Error as PlonkError;
use phoenix_core::Error as PhoenixError;

use std::str::ParseBoolError;
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    PhoenixError(PhoenixError),
    PlonkError(PlonkError),
    NoSuchBranch,
    ParseBoolError(ParseBoolError),
    Io(io::Error),
    KeysNotFound,
    CircuitMaximumInputs,
    CircuitMaximumOutputs,
    SignatureNotComputed,
    IncorrectExecuteCircuitVariant(usize, usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ParseBoolError(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PhoenixError> for Error {
    fn from(e: PhoenixError) -> Self {
        Self::PhoenixError(e)
    }
}

impl From<PlonkError> for Error {
    fn from(e: PlonkError) -> Self {
        Self::PlonkError(e)
    }
}

impl From<ParseBoolError> for Error {
    fn from(e: ParseBoolError) -> Self {
        Self::ParseBoolError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
