// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rocksdb::OptimisticTransactionDB;
use sqlx::sqlite::SqlitePool;
use tracing::debug;

pub mod conf;
mod moonlight;
mod sqlite;
mod transformer;

use conf::Params as ArchiveParams;

pub use moonlight::{MoonlightGroup, Order};
pub use sqlite::SupplyInfo;

// Archive folder containing the sqlite database and the moonlight database
const ARCHIVE_FOLDER_NAME: &str = "archive";

/// The Archive for the archive node.
///
/// The implementation for the sqlite archive and archivist trait is in the
/// `sqlite` module. The implementation for the moonlight database is in the
/// `moonlight` module.
#[derive(Debug, Clone)]
pub struct Archive {
    // Writer pool (single connection) to the SQLite database
    sqlite_writer: SqlitePool,
    // Reader pool (read-only) for GraphQL/queries
    sqlite_reader: SqlitePool,
    // The connection pool can be cloned and stays the same as it is behind an
    // Arc PoolInner. Pool<DB: Database>(pub(crate) Arc<PoolInner<DB>>)
    // The moonlight database.
    moonlight_db: Arc<OptimisticTransactionDB>,
    // last finalized block height known to the archive
    last_finalized_block_height: u64,
}

impl Archive {
    fn archive_folder_path<P: AsRef<Path>>(base_path: P) -> PathBuf {
        let path = base_path.as_ref().join(ARCHIVE_FOLDER_NAME);

        // Recursively create the archive folder if it doesn't exist already
        fs::create_dir_all(&path).unwrap_or_else(|_| {
            panic!("creating directory in {path:?} should not fail")
        });
        path
    }

    /// Create or open the archive database
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path to the base folder where the archive folder
    ///   resides in or will be created.
    pub async fn create_or_open<P: AsRef<Path>>(base_path: P) -> Self {
        Self::create_or_open_with_conf(base_path, ArchiveParams::default())
            .await
    }

    /// Create or open the archive database with configuration parameters
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path to the base folder where the archive folder
    ///   resides in or will be created.
    /// * `params` - Archive node configuration parameters.
    pub async fn create_or_open_with_conf<P: AsRef<Path>>(
        base_path: P,
        params: ArchiveParams,
    ) -> Self {
        let path = Self::archive_folder_path(base_path);

        tracing::info!(
            "Archive::create_or_open_with_conf with conf {}",
            params
        );

        let sqlite_writer = Self::create_writer_pool(&path).await;
        let sqlite_reader =
            Self::create_reader_pool(&path, params.reader_max_connections)
                .await;
        let moonlight_db = Self::create_or_open_moonlight_db(&path, params);

        let mut self_archive = Self {
            sqlite_writer,
            sqlite_reader,
            moonlight_db,
            last_finalized_block_height: 0,
        };

        let last_finalized_block_height = match self_archive
            .fetch_last_finalized_block()
            .await
        {
            Ok((height, _)) => height,
            Err(e) => {
                // If the error is sqlx::Error::RowNotFound then it is fine
                // during sync from scratch
                debug!("Error fetching last finalized block height during archive initialization: {e}");
                0
            }
        };

        self_archive.last_finalized_block_height = last_finalized_block_height;

        self_archive
    }

    /// Returns the last finalized block height cached in the archive.
    ///
    /// # Note
    /// This is defined as the last finalized block height that was stored in
    /// the archive. This means also that it is the last height that the
    /// specific archive is aware of.
    pub fn last_finalized_block_height(&self) -> u64 {
        self.last_finalized_block_height
    }
}
