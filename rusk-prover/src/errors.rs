// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error;
use std::fmt;

#[derive(Debug)]
pub enum ProverError {
    InvalidData {
        field: &'static str,
        inner: dusk_bytes::Error,
    },
    Other(String),
}
impl ProverError {
    pub fn invalid_data(field: &'static str, inner: dusk_bytes::Error) -> Self {
        Self::InvalidData { field, inner }
    }
    pub fn with_context<E: std::error::Error>(
        context: &'static str,
        err: E,
    ) -> Self {
        Self::from(format!("{context} - {err:?}"))
    }
}

impl error::Error for ProverError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl fmt::Display for ProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProverError::InvalidData { field, inner } => {
                write!(f, "Invalid field '{field}': {inner:?}")
            }
            ProverError::Other(context) => write!(f, "{context}"),
        }
    }
}

impl From<String> for ProverError {
    fn from(desc: String) -> Self {
        ProverError::Other(desc)
    }
}
