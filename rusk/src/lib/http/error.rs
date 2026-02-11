// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Client provided invalid input (malformed hex, bad contract ID, etc.)
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Version mismatch or missing version header
    #[error("Version error: {0}")]
    VersionMismatch(String),
    /// Invalid UTF-8 in request body
    #[error("Invalid request encoding: {0}")]
    InvalidEncoding(String),
    /// Requested resource was not found
    #[error("Not found: {0}")]
    NotFound(String),
    /// Unsupported operation / endpoint
    #[error("Unsupported operation")]
    Unsupported,
    /// JSON serialization/deserialization failure
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// VM / contract execution error
    #[error("VM error: {0}")]
    Vm(String),
    /// Database / storage error
    #[error("Database error: {0}")]
    Database(String),
    /// Data driver encode/decode error
    #[error("Data driver error: {0}")]
    DataDriver(String),
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Prover error
    #[error("Prover error: {0}")]
    Prover(String),
    /// Signature / cryptographic verification failure
    #[error("Verification error: {0}")]
    Verification(String),
    /// Catch-all for errors that don't fit other variants
    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub fn http_code(&self) -> u16 {
        match self {
            Error::InvalidInput(_)
            | Error::VersionMismatch(_)
            | Error::InvalidEncoding(_) => 400,
            Error::NotFound(_) => 404,
            Error::Unsupported => 501,
            Error::Serialization(_)
            | Error::Vm(_)
            | Error::Database(_)
            | Error::DataDriver(_)
            | Error::Io(_)
            | Error::Prover(_)
            | Error::Verification(_)
            | Error::Internal(_) => 500,
        }
    }

    pub fn invalid_input<T: AsRef<str>>(msg: T) -> Self {
        Error::InvalidInput(msg.as_ref().to_string())
    }

    pub fn not_found<T: AsRef<str>>(msg: T) -> Self {
        Error::NotFound(msg.as_ref().to_string())
    }

    pub fn vm<T: AsRef<str>>(msg: T) -> Self {
        Error::Vm(msg.as_ref().to_string())
    }

    pub fn database<T: AsRef<str>>(msg: T) -> Self {
        Error::Database(msg.as_ref().to_string())
    }

    pub fn data_driver<T: AsRef<str>>(msg: T) -> Self {
        Error::DataDriver(msg.as_ref().to_string())
    }

    pub fn prover<T: AsRef<str>>(msg: T) -> Self {
        Error::Prover(msg.as_ref().to_string())
    }

    pub fn verification<T: AsRef<str>>(msg: T) -> Self {
        Error::Verification(msg.as_ref().to_string())
    }

    pub fn internal<T: AsRef<str>>(msg: T) -> Self {
        Error::Internal(msg.as_ref().to_string())
    }
}

impl From<dusk_data_driver::Error> for Error {
    fn from(e: dusk_data_driver::Error) -> Self {
        Self::DataDriver(e.to_string())
    }
}

impl From<semver::Error> for Error {
    fn from(e: semver::Error) -> Self {
        Self::VersionMismatch(e.to_string())
    }
}
