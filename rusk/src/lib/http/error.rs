// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Generic error
    #[error("Generic error: {0}")]
    Generic(String),
    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Unsupported operation
    #[error("Unsupported operation")]
    Unsupported,
}

impl Error {
    pub fn http_code(&self) -> u16 {
        match self {
            Error::Generic(_) => 500,
            Error::InvalidInput(_) => 400,
            Error::Unsupported => 501,
        }
    }

    pub fn invalid_input<T: AsRef<str>>(msg: T) -> Self {
        Error::InvalidInput(msg.as_ref().to_string())
    }

    pub fn generic<T: AsRef<str>>(msg: T) -> Self {
        Error::Generic(msg.as_ref().to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::generic(e.to_string())
    }
}
