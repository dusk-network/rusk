// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;
use std::num::ParseFloatError;
use std::path::PathBuf;
use std::str::Utf8Error;

use hex::FromHexError;
use node_data::bls::ConsensusKeysError;
use rand::Error as RngError;
use serde_json::Value;
use url::ParseError;

use crate::gql::GraphQLError;

/// Errors produced while talking to Rusk HTTP or GraphQL endpoints.
#[derive(Debug, thiserror::Error)]
pub enum RuskError {
    /// Invalid base node URI provided to the HTTP client.
    #[error("Invalid base node URI `{uri}`: {source}")]
    InvalidBaseNodeUri {
        /// Raw URI string provided by the caller.
        uri: String,
        /// URL parsing failure for the provided URI.
        #[source]
        source: ParseError,
    },
    /// Invalid node endpoint path derived from the base URI.
    #[error("Invalid node endpoint path `{path}`: {source}")]
    InvalidEndpointPath {
        /// Relative endpoint path that failed to join.
        path: String,
        /// URL parsing failure for the joined endpoint.
        #[source]
        source: ParseError,
    },
    /// Remote Rusk service returned an error payload.
    #[error("Rusk service returned an error: {message}")]
    Response {
        /// Response body or synthesized error text returned by the service.
        message: String,
    },
    /// GraphQL response contained an `errors` payload.
    #[error("GraphQL response contained errors: {errors}")]
    GraphqlErrors {
        /// Raw GraphQL `errors` payload returned by the service.
        errors: Value,
    },
    /// GraphQL response did not contain the expected `data` field.
    #[error("GraphQL response missing data")]
    MissingGraphqlData,
}

/// Errors reported by transaction submission or confirmation flows.
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    /// The transaction was rejected as a replay.
    #[error("Replay")]
    Replay,
    /// The node rejected the transaction with a message.
    #[error("{message}")]
    Rejected {
        /// Free-form rejection message returned by the node.
        message: String,
    },
}

/// Errors surfaced while proving Phoenix transactions.
#[derive(Debug, thiserror::Error)]
pub enum ProverError {
    /// Circuit generation or verification failed.
    #[error("{message}")]
    Circuit {
        /// Error text returned by the proving stack.
        message: String,
    },
    /// External prover invocation failed.
    #[error("{message}")]
    Backend {
        /// Error text returned by the proving backend.
        message: String,
    },
}

/// Errors reported while preparing or validating blob inputs.
#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    /// Blob transactions were requested for a shielded sender.
    #[error("Blob is unsupported for Shielded Addresses")]
    UnsupportedShieldedAddress,
    /// Blob input file could not be read.
    #[error("Invalid path {}: {source}", path.display())]
    InvalidPath {
        /// Blob file path that could not be read.
        path: PathBuf,
        /// Filesystem error returned while reading the blob.
        #[source]
        source: io::Error,
    },
    /// Blob file contained invalid hex data.
    #[error("Invalid hex in {}: {source}", path.display())]
    InvalidHex {
        /// Blob file path that contained invalid hex bytes.
        path: PathBuf,
        /// Hex decoding failure.
        #[source]
        source: FromHexError,
    },
    /// Blob payload could not be encoded as a data part.
    #[error("{message}")]
    InvalidDataPart {
        /// Data part validation message.
        message: String,
    },
    /// Blob failure reported by an upstream dependency.
    #[error("{message}")]
    Upstream {
        /// Free-form error text returned by the upstream dependency.
        message: String,
    },
}

/// Errors converting into or parsing DUSK values.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// DUSK values cannot be negative.
    #[error("Dusk type does not support negative values")]
    NegativeDuskValue,
    /// String input could not be parsed as a DUSK amount.
    #[error("Failed to parse Dusk from string: {source}")]
    ParseDusk {
        /// Float parser error for the original input.
        #[source]
        source: ParseFloatError,
    },
}

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
    #[error(transparent)]
    Rusk(#[from] RuskError),
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
    /// The note couldn't be decrypted with the provided `ViewKey`
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
    /// Invalid endpoint URL override
    #[error("Invalid {endpoint} endpoint URL `{value}`: {source}")]
    InvalidEndpointUrl {
        /// Endpoint name being overridden.
        endpoint: &'static str,
        /// Raw override value provided by the user.
        value: String,
        #[source]
        /// URL parsing failure for the provided override.
        source: ParseError,
    },
    /// Insecure transport was requested without explicit opt-in
    #[error("Refusing insecure HTTP connection to {0}")]
    InsecureTransport(String),
    /// Endpoint URL uses an unsupported scheme.
    #[error(
        "Unsupported {endpoint} endpoint URL scheme `{scheme}` in `{value}`; expected `https` or `http`"
    )]
    UnsupportedEndpointScheme {
        /// Endpoint name being configured.
        endpoint: &'static str,
        /// Unsupported URL scheme.
        scheme: String,
        /// URL value provided by the user or config.
        value: String,
    },
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
    #[error(transparent)]
    Transaction(#[from] TransactionError),
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
    #[error(transparent)]
    ProverError(#[from] ProverError),
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
    /// Error while processing blob data
    #[error(transparent)]
    Blob(#[from] BlobError),
    /// Conversion error
    #[error(transparent)]
    Conversion(#[from] ConversionError),
    /// GraphQL error
    #[error("GraphQL error: {0}")]
    GraphQLError(GraphQLError),
    /// Archive node request failed
    #[error("Archive node request failed: {0}")]
    ArchiveQuery(#[source] Box<Error>),
    /// Archive node JSON decoding failed
    #[error("Archive node JSON decoding failed: {0}")]
    ArchiveJson(#[source] serde_json::Error),
    /// Consensus keys error
    #[error("Error while saving consensus keys: {0}")]
    ConsensusKeysError(ConsensusKeysError),
    /// Note cache is from a different chain; delete the cache directory and
    /// restart
    #[error(
        "Stale note cache at position {0}; delete the cache directory and restart."
    )]
    StaleCache(u64),
    /// Trying to claim more reward than the person has
    #[error("Trying to claim more than existing reward")]
    NotEnoughReward,
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
        match e {
            dusk_core::Error::InsufficientBalance => Self::NotEnoughBalance,
            dusk_core::Error::Replay => {
                Self::Transaction(TransactionError::Replay)
            }
            dusk_core::Error::PhoenixOwnership => Self::AddressNotOwned,
            dusk_core::Error::PhoenixCircuit(message) => {
                Self::ProverError(ProverError::Circuit { message })
            }
            dusk_core::Error::PhoenixProver(message) => {
                Self::ProverError(ProverError::Backend { message })
            }
            dusk_core::Error::InvalidData => {
                Self::Bytes(dusk_bytes::Error::InvalidData)
            }
            dusk_core::Error::BadLength(found, expected) => {
                Self::Bytes(dusk_bytes::Error::BadLength { found, expected })
            }
            dusk_core::Error::InvalidChar(ch, index) => {
                Self::Bytes(dusk_bytes::Error::InvalidChar { ch, index })
            }
            dusk_core::Error::Rkyv(_) => Self::Rkyv,
            dusk_core::Error::MemoTooLarge(m) => Self::MemoTooLarge(m),
            dusk_core::Error::Blob(message) => {
                Self::Blob(BlobError::Upstream { message })
            }
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

impl From<node_data::bls::ConsensusKeysError> for Error {
    fn from(e: ConsensusKeysError) -> Self {
        Self::ConsensusKeysError(e)
    }
}
