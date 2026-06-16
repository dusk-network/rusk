// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;

/// Typed status updates emitted by wallet operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletStatus {
    /// Informational status text for an in-flight wallet operation.
    Info(String),
    /// Warning text for a degraded but non-fatal wallet condition.
    Warning(String),
    /// Structured wallet sync progress update.
    Sync(WalletSyncStatus),
}

impl From<WalletSyncStatus> for WalletStatus {
    fn from(status: WalletSyncStatus) -> Self {
        Self::Sync(status)
    }
}

impl fmt::Display for WalletStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info(message) | Self::Warning(message) => {
                write!(f, "{message}")
            }
            Self::Sync(status) => write!(f, "{status}"),
        }
    }
}

/// Structured sync progress updates emitted by wallet state synchronization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletSyncStatus {
    /// A sync session has started.
    Starting,
    /// The wallet is reading the cached note position.
    ReadingCachedNotePosition,
    /// The wallet is resuming from a cached note position.
    ResumingFromCachedNotePosition(u64),
    /// The wallet is requesting fresh notes from the node.
    FetchingFreshNotes,
    /// The note stream connection has been established.
    NoteStreamConnected,
    /// The wallet is streaming notes from the node.
    StreamingNotes,
    /// The wallet has advanced to the given block height while syncing.
    StreamingProgress(u64),
    /// The wallet detected a stale cache and is resetting it.
    CacheResetting,
    /// Sync completed at the given block height and note position.
    Complete(u64, u64),
    /// Sync failed with the provided message.
    Error(String),
}

impl fmt::Display for WalletSyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting => write!(f, "Starting sync session..."),
            Self::ReadingCachedNotePosition => {
                write!(f, "Getting cached note position...")
            }
            Self::ResumingFromCachedNotePosition(note_position) => write!(
                f,
                "Resuming sync from cached note position {note_position}",
            ),
            Self::FetchingFreshNotes => {
                write!(f, "Fetching fresh notes...")
            }
            Self::NoteStreamConnected => {
                write!(f, "Connection established...")
            }
            Self::StreamingNotes => write!(f, "Streaming notes..."),
            Self::StreamingProgress(block_height) => {
                write!(f, "Syncing chain state at block {block_height}")
            }
            Self::CacheResetting => {
                write!(f, "Stale cache detected; resetting note cache...")
            }
            Self::Complete(block_height, note_position) => write!(
                f,
                "Syncing complete at block {block_height} (note position {note_position})",
            ),
            Self::Error(message) => {
                write!(f, "Error during sync: {message}")
            }
        }
    }
}
