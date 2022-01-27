// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use block_modes::{BlockModeError, InvalidKeyIvLength};
use canonical::CanonError;
use std::io;
use tonic::Status;

use super::clients;

pub type CoreError =
    dusk_wallet_core::Error<crate::LocalStore, clients::State, clients::Prover>;

/// Errors returned by this crate
#[derive(Debug)]
pub enum CliError {
    InvalidPhrase,
    FileCorrupted,
    FileNotExists,
    FileExists,

    Network(tonic::transport::Error),
    Connection(tonic::Status),

    CryptoKey(InvalidKeyIvLength),
    CryptoBlock(BlockModeError),

    Bytes(dusk_bytes::Error),
    Base58(bs58::decode::Error),
    Canon(CanonError),
    IO(io::Error),

    WalletCore(Box<CoreError>),
}

impl From<dusk_bytes::Error> for CliError {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<CanonError> for CliError {
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

impl From<tonic::transport::Error> for CliError {
    fn from(e: tonic::transport::Error) -> Self {
        Self::Network(e)
    }
}

impl From<bs58::decode::Error> for CliError {
    fn from(e: bs58::decode::Error) -> Self {
        Self::Base58(e)
    }
}

impl From<CoreError> for CliError {
    fn from(e: CoreError) -> Self {
        Self::WalletCore(Box::new(e))
    }
}

impl From<InvalidKeyIvLength> for CliError {
    fn from(e: InvalidKeyIvLength) -> Self {
        Self::CryptoKey(e)
    }
}

impl From<BlockModeError> for CliError {
    fn from(e: BlockModeError) -> Self {
        Self::CryptoBlock(e)
    }
}

// Todo: Rename CliError -> Error
