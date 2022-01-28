// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{fmt, io};

use super::clients;
pub type CoreError =
    dusk_wallet_core::Error<crate::LocalStore, clients::State, clients::Prover>;

/// Errors returned by this crate
pub enum Error {
    /// Recovery phrase is not valid
    InvalidMnemonicPhrase,
    /// Wallet file content is not valid
    WalletFileCorrupted,
    /// Wallet file not found on disk
    WalletFileNotExists,
    /// A wallet file with this name already exists
    WalletFileExists,
    /// Network error while communicating with rusk
    Network(tonic::transport::Error),
    /// Connection error with rusk
    Connection(tonic::Status),
    /// Wrong wallet password
    InvalidPassword,
    /// Bytes encoding errors
    Bytes(dusk_bytes::Error),
    /// Base58 errors
    Base58(bs58::decode::Error),
    /// Canonical errors
    Canon(canonical::CanonError),
    /// Filesystem errors
    IO(io::Error),
    /// Wallet Core lib errors
    WalletCore(Box<CoreError>),
    /// User graceful exit
    UserExit,
}

impl From<dusk_bytes::Error> for Error {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<canonical::CanonError> for Error {
    fn from(e: canonical::CanonError) -> Self {
        Self::Canon(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<tonic::Status> for Error {
    fn from(s: tonic::Status) -> Self {
        Self::Connection(s)
    }
}

impl From<tonic::transport::Error> for Error {
    fn from(e: tonic::transport::Error) -> Self {
        Self::Network(e)
    }
}

impl From<bs58::decode::Error> for Error {
    fn from(e: bs58::decode::Error) -> Self {
        Self::Base58(e)
    }
}

impl From<CoreError> for Error {
    fn from(e: CoreError) -> Self {
        Self::WalletCore(Box::new(e))
    }
}

impl From<block_modes::InvalidKeyIvLength> for Error {
    fn from(_: block_modes::InvalidKeyIvLength) -> Self {
        Self::WalletFileCorrupted
    }
}

impl From<block_modes::BlockModeError> for Error {
    fn from(_: block_modes::BlockModeError) -> Self {
        Self::InvalidPassword
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidMnemonicPhrase => {
                write!(f, "Recovery phrase is not valid")
            }
            Error::WalletFileCorrupted => {
                write!(f, "Wallet file content is not valid")
            }
            Error::WalletFileNotExists => {
                write!(f, "Wallet file not found on disk")
            }
            Error::WalletFileExists => {
                write!(f, "A wallet file with this name already exists")
            }
            Error::Network(err) => {
                write!(
                    f,
                    "Network error while communicating with rusk: {}",
                    err
                )
            }
            Error::Connection(err) => {
                write!(f, "Connection error with rusk: {}", err)
            }
            Error::InvalidPassword => {
                write!(f, "Wrong wallet password")
            }
            Error::Bytes(err) => {
                write!(f, "Bytes encoding errors: {:?}", err)
            }
            Error::Base58(err) => {
                write!(f, "Base58 errors: {}", err)
            }
            Error::Canon(err) => {
                write!(f, "Canonical errors: {:?}", err)
            }
            Error::IO(err) => {
                write!(f, "Filesystem errors: {}", err)
            }
            Error::WalletCore(err) => {
                write!(f, "Wallet Core lib errors: {:?}", err)
            }
            Error::UserExit => {
                write!(f, "Done")
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidMnemonicPhrase => {
                write!(f, "Recovery phrase is not valid")
            }
            Error::WalletFileCorrupted => {
                write!(f, "Wallet file content is not valid")
            }
            Error::WalletFileNotExists => {
                write!(f, "Wallet file not found on disk")
            }
            Error::WalletFileExists => {
                write!(f, "A wallet file with this name already exists")
            }
            Error::Network(err) => {
                write!(
                    f,
                    "Network error while communicating with rusk: {}",
                    err
                )
            }
            Error::Connection(err) => {
                write!(f, "Connection error with rusk: {}", err)
            }
            Error::InvalidPassword => {
                write!(f, "Wrong wallet password")
            }
            Error::Bytes(err) => {
                write!(f, "Bytes encoding errors: {:?}", err)
            }
            Error::Base58(err) => {
                write!(f, "Base58 errors: {}", err)
            }
            Error::Canon(err) => {
                write!(f, "Canonical errors: {:?}", err)
            }
            Error::IO(err) => {
                write!(f, "Filesystem errors: {}", err)
            }
            Error::WalletCore(err) => {
                write!(f, "Wallet Core lib errors: {:?}", err)
            }
            Error::UserExit => {
                write!(f, "Done")
            }
        }
    }
}
