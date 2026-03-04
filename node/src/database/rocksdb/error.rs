// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum RocksDbError {
    #[error("Cannot read tip")]
    CannotReadTip,
    #[error("Cannot read block {hash}")]
    CannotReadBlock { hash: String },
    #[error("Cannot find tip stored in metadata")]
    TipMetadataMissing,
    #[error("Cannot find tip block")]
    TipBlockMissing,
    #[error("Failed to parse blob sidecar: {details}")]
    BlobSidecarParse { details: String },
    #[error("At least one Transaction ID was not found")]
    MissingLedgerTransaction,
    #[error("no value")]
    MissingIteratorValue,
    #[error("invalid data")]
    InvalidTimestampData,
    #[error("failed to commit")]
    Commit {
        #[source]
        source: rocksdb::Error,
    },
    #[error("failed to rollback")]
    Rollback {
        #[source]
        source: rocksdb::Error,
    },
}

impl RocksDbError {
    pub(super) fn cannot_read_block(hash: &[u8]) -> Self {
        Self::CannotReadBlock {
            hash: hex::encode(hash),
        }
    }

    pub(super) fn blob_sidecar_parse<E: std::fmt::Debug>(error: E) -> Self {
        Self::BlobSidecarParse {
            details: format!("{error:?}"),
        }
    }

    pub(super) fn commit_failed(source: rocksdb::Error) -> Self {
        Self::Commit { source }
    }

    pub(super) fn rollback_failed(source: rocksdb::Error) -> Self {
        Self::Rollback { source }
    }
}
