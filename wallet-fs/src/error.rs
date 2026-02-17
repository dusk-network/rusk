// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use block_modes::{BlockModeError, InvalidKeyIvLength};

/// Errors that occur during file & key management operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failure to encrypt or decrypt data.
    #[error("Failed to encrypt/decrypt")]
    EncryptDecryptFailure,
    /// The data is not valid.
    #[error("The data is corrupted")]
    CorruptedData,
    /// JSON serialization/deserialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Invalid consensus keys file path.
    #[error(
        "Expected the consensus keys file path to be a valid UTF-8 string, have a filename and a parent directory"
    )]
    InvalidKeysFilePath,
}

impl From<aes_gcm::Error> for Error {
    fn from(_err: aes_gcm::Error) -> Self {
        Error::EncryptDecryptFailure
    }
}

impl From<InvalidKeyIvLength> for Error {
    fn from(_err: InvalidKeyIvLength) -> Self {
        Error::CorruptedData
    }
}

impl From<BlockModeError> for Error {
    fn from(_err: BlockModeError) -> Self {
        Error::EncryptDecryptFailure
    }
}
