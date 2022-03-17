// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phoenix_core::Error as PhoenixError;
use rand_core::Error as RngError;
use std::{fmt, io};

use super::clients;

/// Wallet core error
pub type CoreError =
    dusk_wallet_core::Error<crate::LocalStore, clients::State, clients::Prover>;

/// Errors returned by this crate
pub enum Error {
    /// State Client errors
    State(StateError),
    /// Prover Client errors
    Prover(ProverError),
    /// Local Store errors
    Store(StoreError),
    /// Network error while communicating with rusk
    Network(tonic::transport::Error),
    /// Command not available in offline mode
    Offline,
    /// Filesystem errors
    IO(io::Error),
    /// JSON serialization errors
    JSON(serde_json::Error),
    /// TOML deserialization errors
    ConfigRead(toml::de::Error),
    /// TOML serialization errors
    ConfigWrite(toml::ser::Error),
    /// Bytes encoding errors
    Bytes(dusk_bytes::Error),
    /// Base58 errors
    Base58(bs58::decode::Error),
    /// Canonical errors
    Canon(canonical::CanonError),
    /// Random number generator errors
    Rng(RngError),
    /// Transaction model errors
    Phoenix(PhoenixError),
    /// Not enough balance to perform transaction.
    NotEnoughBalance,
    /// Note combination for the given value is impossible given the maximum
    /// amount if inputs in a transaction.
    NoteCombinationProblem,
    /// Not enough gas to perform this transaction
    NotEnoughGas,
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::JSON(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Self::ConfigRead(e)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Self::ConfigWrite(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<dusk_bytes::Error> for Error {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
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

impl From<StateError> for Error {
    fn from(e: StateError) -> Self {
        Self::State(e)
    }
}

impl From<ProverError> for Error {
    fn from(e: ProverError) -> Self {
        Self::Prover(e)
    }
}

impl From<StoreError> for Error {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<CoreError> for Error {
    fn from(e: CoreError) -> Self {
        use dusk_wallet_core::Error as CoreErr;
        match e {
            CoreErr::Store(err) => Self::Store(err),
            CoreErr::State(err) => Self::State(err),
            CoreErr::Prover(err) => Self::Prover(err),
            CoreErr::Canon(err) => Self::Canon(err),
            CoreErr::Rng(err) => Self::Rng(err),
            CoreErr::Bytes(err) => Self::Bytes(err),
            CoreErr::Phoenix(err) => Self::Phoenix(err),
            CoreErr::NotEnoughBalance => Self::NotEnoughBalance,
            CoreErr::NoteCombinationProblem => Self::NoteCombinationProblem,
        }
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::State(err) => write!(f, "\r{}", err),
            Error::Prover(err) => write!(f, "\r{}", err),
            Error::Store(err) => write!(f, "\r{}", err),
            Error::Network(err) => write!(f, "\rA network error occurred while communicating with Rusk:\n{}", err),
            Error::Offline => write!(f, "\rThis command cannot be performed while offline. Please configure a valid Rusk instance and try again."),
            Error::IO(err) => write!(f, "\rAn IO error occurred:\n{}", err),
            Error::JSON(err) => write!(f, "\rA serialization error occurred:\n{}", err),
            Error::ConfigRead(err) => write!(f, "\rFailed to read configuration file:\n{}", err),
            Error::ConfigWrite(err) => write!(f, "\rFailed to write to configuration file:\n{}", err),
            Error::Bytes(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::Base58(err) => write!(f, "\rA serialization error occurred:\n{}", err),
            Error::Canon(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::Rng(err) => write!(f, "\rAn error occured while using the random number generator:\n{}", err),
            Error::Phoenix(err) => write!(f, "\rAn error occured in Phoenix:\n{}", err),
            Error::NotEnoughGas => write!(f, "\rNot enough gas to perform this transaction"),
            Error::NotEnoughBalance => write!(f, "\rInsufficient balance to perform this operation"),
            Error::NoteCombinationProblem => write!(f, "\rNote combination for the given value is impossible given the maximum amount of inputs in a transaction"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::State(err) => write!(f, "\r{:?}", err),
            Error::Prover(err) => write!(f, "\r{:?}", err),
            Error::Store(err) => write!(f, "\r{:?}", err),
            Error::Network(err) => write!(f, "\rA network error occurred while communicating with Rusk:\n{:?}", err),
            Error::Offline => write!(f, "\rThis command cannot be performed while offline. Please configure a valid Rusk instance and try again."),
            Error::IO(err) => write!(f, "\rAn IO error occurred:\n{:?}", err),
            Error::JSON(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::ConfigRead(err) => write!(f, "\rFailed to read configuration file:\n{:?}", err),
            Error::ConfigWrite(err) => write!(f, "\rFailed to write to configuration file:\n{:?}", err),
            Error::Bytes(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::Base58(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::Canon(err) => write!(f, "\rA serialization error occurred:\n{:?}", err),
            Error::Rng(err) => write!(f, "\rAn error occured while using the random number generator:\n{:?}", err),
            Error::Phoenix(err) => write!(f, "\rAn error occured in Phoenix:\n{:?}", err),
            Error::NotEnoughGas => write!(f, "\rNot enough gas to perform this transaction"),
            Error::NotEnoughBalance => write!(f, "\rInsufficient balance to perform this operation"),
            Error::NoteCombinationProblem => write!(f, "\rNote combination for the given value is impossible given the maximum amount of inputs in a transaction"),
        }
    }
}

/// State client errors
pub enum StateError {
    /// Status of a Rusk request
    Rusk(String),
    /// Bytes encoding errors
    Bytes(dusk_bytes::Error),
    /// Canonical errors
    Canon(canonical::CanonError),
}

impl From<dusk_bytes::Error> for StateError {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<canonical::CanonError> for StateError {
    fn from(e: canonical::CanonError) -> Self {
        Self::Canon(e)
    }
}

impl From<tonic::Status> for StateError {
    fn from(s: tonic::Status) -> Self {
        Self::Rusk(s.message().to_string())
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StateError::Rusk(st) => {
                write!(f, "\rRusk returned an error:\n{}", st)
            }
            StateError::Bytes(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
            StateError::Canon(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
        }
    }
}

impl fmt::Debug for StateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StateError::Rusk(st) => {
                write!(f, "\rRusk returned an error:\n{:?}", st)
            }
            StateError::Bytes(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
            StateError::Canon(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
        }
    }
}

/// Prover client errors
pub enum ProverError {
    /// Status of a Rusk request
    Rusk(String),
    /// Bytes encoding errors
    Bytes(dusk_bytes::Error),
    /// Canonical errors
    Canon(canonical::CanonError),
}

impl From<dusk_bytes::Error> for ProverError {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<canonical::CanonError> for ProverError {
    fn from(e: canonical::CanonError) -> Self {
        Self::Canon(e)
    }
}

impl From<tonic::Status> for ProverError {
    fn from(s: tonic::Status) -> Self {
        Self::Rusk(s.message().to_string())
    }
}

impl fmt::Display for ProverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProverError::Rusk(st) => {
                write!(f, "\rRusk returned an error:\n{}", st)
            }
            ProverError::Bytes(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
            ProverError::Canon(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
        }
    }
}

impl fmt::Debug for ProverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProverError::Rusk(st) => {
                write!(f, "\rRusk returned an error:\n{:?}", st)
            }
            ProverError::Bytes(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
            ProverError::Canon(err) => {
                write!(f, "\rA serialization error occurred:\n{:?}", err)
            }
        }
    }
}

/// Store errors
pub enum StoreError {
    /// Wallet file content is not valid
    WalletFileCorrupted,
    /// File version not recognized
    UnknownFileVersion,
    /// Wallet file not found on disk
    WalletFileNotExists,
    /// A wallet file with this name already exists
    WalletFileExists,
    /// Wrong wallet password
    InvalidPassword,
    /// Recovery phrase is not valid
    InvalidMnemonicPhrase,
    /// Filesystem errors
    IO(io::Error),
}

impl From<block_modes::InvalidKeyIvLength> for StoreError {
    fn from(_: block_modes::InvalidKeyIvLength) -> Self {
        Self::WalletFileCorrupted
    }
}

impl From<block_modes::BlockModeError> for StoreError {
    fn from(_: block_modes::BlockModeError) -> Self {
        Self::InvalidPassword
    }
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StoreError::InvalidMnemonicPhrase => {
                write!(f, "\rInvalid recovery phrase")
            }
            StoreError::WalletFileCorrupted => {
                write!(f, "\rWallet file content is not valid")
            }
            StoreError::UnknownFileVersion => {
                write!(f, "\rFile version not recognized")
            }
            StoreError::WalletFileNotExists => {
                write!(f, "\rWallet file not found on disk")
            }
            StoreError::WalletFileExists => {
                write!(f, "\rA wallet file with this name already exists")
            }
            StoreError::InvalidPassword => write!(f, "\rWrong password"),
            StoreError::IO(err) => {
                write!(f, "\rAn IO error occurred:\n{}", err)
            }
        }
    }
}

impl fmt::Debug for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StoreError::InvalidMnemonicPhrase => {
                write!(f, "\rInvalid recovery phrase")
            }
            StoreError::WalletFileCorrupted => {
                write!(f, "\rWallet file content is not valid")
            }
            StoreError::UnknownFileVersion => {
                write!(f, "\rFile version not recognized")
            }
            StoreError::WalletFileNotExists => {
                write!(f, "\rWallet file not found on disk")
            }
            StoreError::WalletFileExists => {
                write!(f, "\rA wallet file with this name already exists")
            }
            StoreError::InvalidPassword => write!(f, "\rWrong password"),
            StoreError::IO(err) => {
                write!(f, "\rAn IO error occurred:\n{:?}", err)
            }
        }
    }
}
