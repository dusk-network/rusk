// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;
use std::str::Utf8Error;

use hex::FromHexError;
use inquire::InquireError;
use node_data::bls::ConsensusKeysError;
use rand::Error as RngError;

use crate::gql::GraphQLError;

/// Errors returned by this library
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Command not available in offline mode
    #[error("This command cannot be performed while offline")]
    Offline,
    /// Unauthorized access to this address
    #[error("Unauthorized access to this address")]
    Unauthorized,
    /// Rusk error
    #[error("Rusk error occurred: {0}")]
    Rusk(String),
    /// Filesystem errors
    #[error(transparent)]
    IO(#[from] io::Error),
    /// JSON serialization errors
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Bytes encoding errors
    #[error("A serialization error occurred: {0:?}")]
    Bytes(dusk_bytes::Error),
    /// Base58 errors
    #[error(transparent)]
    Base58(#[from] bs58::decode::Error),
    /// Rkyv errors
    #[error("A serialization error occurred.")]
    Rkyv,
    /// Hex errors
    #[error("Invalid Hex data: {0}")]
    Hex(#[from] FromHexError),
    /// Error creating HTTP client
    #[error("Cannot create HTTP client")]
    HttpClient,
    /// Reqwest errors
    #[error("A request error occurred: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// Utf8 errors
    #[error("Utf8 error: {0:?}")]
    Utf8(Utf8Error),
    /// Random number generator errors
    #[error(transparent)]
    Rng(#[from] RngError),
    /// Not enough balance to perform transaction
    #[error("Insufficient balance to perform this operation")]
    NotEnoughBalance,
    /// Amount to transfer/stake cannot be zero
    #[error("Amount to transfer/stake cannot be zero")]
    AmountIsZero,
    /// Note combination for the given value is impossible given the maximum
    /// amount of inputs in a transaction
    #[error("Impossible notes' combination for the given value is")]
    NoteCombinationProblem,
    /// The note wasn't found in the note-tree of the transfer-contract
    #[error("Note wasn't found in transfer-contract")]
    NoteNotFound,
    /// The note couldn't be decrypted with the provided ViewKey
    #[error("Note couldn't be decrypted with the provided ViewKey")]
    WrongViewKey,
    /// Not enough gas to perform this transaction
    #[error("Not enough gas to perform this transaction")]
    NotEnoughGas,
    /// A stake does not exist for this key
    #[error("A stake does not exist for this key")]
    NotStaked,
    /// No reward available for this key
    #[error("No reward available for this key")]
    NoReward,
    /// Invalid address
    #[error("Invalid address")]
    BadAddress,
    /// Address does not belong to this wallet
    #[error("Address does not belong to this wallet")]
    AddressNotOwned,
    /// No menu item selected
    #[error("No menu item selected")]
    NoMenuItemSelected,
    /// Mnemonic phrase is not valid
    #[error("Invalid mnemonic phrase")]
    InvalidMnemonicPhrase,
    /// Path provided is not a directory
    #[error("Path provided is not a directory")]
    NotDirectory,
    /// Cannot get the path to the $HOME directory
    #[error("OS not supported")]
    OsNotSupported,
    /// Wallet file content is not valid
    #[error("Wallet file content is not valid")]
    WalletFileCorrupted,
    /// File version not recognized
    #[error("File version {0}.{1} not recognized")]
    UnknownFileVersion(u8, u8),
    /// A wallet file with this name already exists
    #[error("A wallet file with this name already exists")]
    WalletFileExists,
    /// Wallet file is missing
    #[error("Wallet file is missing")]
    WalletFileMissing,
    /// Wrong wallet password
    #[error("Invalid password")]
    BlockMode(#[from] block_modes::BlockModeError),
    /// Encryption error from encrypting with AES-GCM for new wallet dat format
    #[error("Encryption error: {0}")]
    Encryption(#[from] aes_gcm::Error),
    /// Reached the maximum number of attempts
    #[error("Reached the maximum number of attempts")]
    AttemptsExhausted,
    /// Status callback needs to be set before connecting
    #[error("Status callback needs to be set before connecting")]
    StatusWalletConnected,
    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),
    /// Rocksdb cache database error
    #[error("Rocks cache database error: {0}")]
    RocksDB(rocksdb::Error),
    /// Provided Network not found
    #[error(
        "Network not found, check config.toml, specify network with -n flag"
    )]
    NetworkNotFound,
    /// The cache database couldn't find column family required
    #[error("Cache database corrupted")]
    CacheDatabaseCorrupted,
    /// Prover errors from dusk-core
    #[error("Prover Error: {0}")]
    ProverError(String),
    /// Memo provided is too large
    #[error("Memo too large {0}")]
    MemoTooLarge(usize),
    /// Expected BLS Key
    #[error("Expected BLS Public Key")]
    ExpectedBlsPublicKey,
    /// Expected Phoenix public key
    #[error("Expected Phoenix public Key")]
    ExpectedPhoenixPublicKey,
    /// Addresses use different transaction models
    #[error("Addresses use different transaction models")]
    DifferentTransactionModels,
    /// Invalid contract id provided
    #[error("Invalid contractID provided")]
    InvalidContractId,
    /// Contract file location not found
    #[error("Invalid WASM contract path provided")]
    InvalidWasmContractPath,
    /// Invalid environment variable value
    #[error("Invalid environment variable value {0}")]
    InvalidEnvVar(String),
    /// Conversion error
    #[error("Conversion error: {0}")]
    Conversion(String),
    /// GraphQL error
    #[error("GraphQL error: {0}")]
    GraphQLError(GraphQLError),
    /// Inquire error
    #[error("Inquire error: {0}")]
    InquireError(String),
    /// Error while querying archival node
    #[error("Archive node query error: {0}")]
    ArchiveJsonError(String),
    /// Consensus keys error
    #[error("Error while saving consensus keys: {0}")]
    ConsensusKeysError(ConsensusKeysError),
}

impl From<dusk_bytes::Error> for Error {
    fn from(e: dusk_bytes::Error) -> Self {
        Self::Bytes(e)
    }
}

impl From<block_modes::InvalidKeyIvLength> for Error {
    fn from(_: block_modes::InvalidKeyIvLength) -> Self {
        Self::WalletFileCorrupted
    }
}

impl From<dusk_core::Error> for Error {
    fn from(e: dusk_core::Error) -> Self {
        use dusk_core::Error::*;

        match e {
            InsufficientBalance => Self::NotEnoughBalance,
            Replay => Self::Transaction("Replay".to_string()),
            PhoenixOwnership => Self::AddressNotOwned,
            PhoenixCircuit(s) | PhoenixProver(s) => Self::ProverError(s),
            InvalidData => Self::Bytes(dusk_bytes::Error::InvalidData),
            BadLength(found, expected) => {
                Self::Bytes(dusk_bytes::Error::BadLength { found, expected })
            }
            InvalidChar(ch, index) => {
                Self::Bytes(dusk_bytes::Error::InvalidChar { ch, index })
            }
            Rkyv(_) => Self::Rkyv,
            MemoTooLarge(m) => Self::MemoTooLarge(m),
        }
    }
}

impl From<rocksdb::Error> for Error {
    fn from(e: rocksdb::Error) -> Self {
        Self::RocksDB(e)
    }
}

impl From<GraphQLError> for Error {
    fn from(e: GraphQLError) -> Self {
        Self::GraphQLError(e)
    }
}

impl From<InquireError> for Error {
    fn from(e: InquireError) -> Self {
        Self::InquireError(e.to_string())
    }
}

impl From<node_data::bls::ConsensusKeysError> for Error {
    fn from(e: ConsensusKeysError) -> Self {
        Self::ConsensusKeysError(e)
    }
}
