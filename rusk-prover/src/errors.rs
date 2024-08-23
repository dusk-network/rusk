// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::string::String;
use core::fmt;

use dusk_plonk::prelude::Error as PlonkError;

#[derive(Debug)]
pub enum ProverError {
    InvalidData {
        field: &'static str,
        inner: dusk_bytes::Error,
    },
    Plonk(PlonkError),
    Other(String),
}

impl ProverError {
    pub fn invalid_data(field: &'static str, inner: dusk_bytes::Error) -> Self {
        Self::InvalidData { field, inner }
    }

    #[cfg(feature = "std")]
    pub fn with_context<E: std::error::Error>(
        context: &'static str,
        err: E,
    ) -> Self {
        Self::from(format!("{context} - {err:?}"))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProverError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<PlonkError> for ProverError {
    fn from(e: PlonkError) -> ProverError {
        ProverError::Plonk(e)
    }
}

impl fmt::Display for ProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProverError::InvalidData { field, inner } => {
                write!(f, "Invalid field '{field}': {inner:?}")
            }
            ProverError::Plonk(plonk_error) => write!(f, "{:?}", plonk_error),
            ProverError::Other(context) => write!(f, "{context}"),
        }
    }
}

impl From<String> for ProverError {
    fn from(desc: String) -> Self {
        ProverError::Other(desc)
    }
}
