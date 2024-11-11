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

mod archivist;
mod moonlight;
mod sqlite;
mod transformer;

pub use archivist::ArchivistSrv;
pub use moonlight::MoonlightGroup;

// Archive folder containing the sqlite database and the moonlight database
const ARCHIVE_FOLDER_NAME: &str = "archive";

/// The Archive for the archive node.
///
/// The implementation for the sqlite archive and archivist trait is in the
/// `sqlite` module. The implementation for the moonlight database is in the
/// `moonlight` module.
#[derive(Debug, Clone)]
pub struct Archive {
    // The connection pool to the sqlite database.
    // The connection pool can be cloned and stays the same as it is behind an
    // Arc PoolInner. Pool<DB: Database>(pub(crate) Arc<PoolInner<DB>>)
    sqlite_archive: SqlitePool,
    // The moonlight database.
    moonlight_db: Arc<OptimisticTransactionDB>,
}

impl Archive {
    fn archive_folder_path<P: AsRef<Path>>(base_path: P) -> PathBuf {
        let path = base_path.as_ref().join(ARCHIVE_FOLDER_NAME);

        // Recursively create the archive folder if it doesn't exist already
        fs::create_dir_all(&path)
            .expect("creating directory in {path} should not fail");
        path
    }

    /// Create or open the archive database
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path to the base folder where the archive folder
    ///   resides in or will be created.
    pub async fn create_or_open<P: AsRef<Path>>(base_path: P) -> Self {
        let path = Self::archive_folder_path(base_path);

        let sqlite_archive = Self::create_or_open_sqlite(&path).await;
        let moonlight_db =
            Self::create_or_open_moonlight_db(&path, ArchiveOptions::default());
        Self {
            sqlite_archive,
            moonlight_db,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArchiveOptions {
    /// Max write buffer size for moonlight event CF.
    pub events_cf_max_write_buffer_size: usize,

    /// Block Cache is useful in optimizing DB reads.
    pub events_cf_disable_block_cache: bool,

    /// Enables a set of flags for collecting DB stats as log data.
    pub enable_debug: bool,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            events_cf_max_write_buffer_size: 1024 * 1024, // 1 MiB
            events_cf_disable_block_cache: false,         // Enable block cache
            enable_debug: false,
        }
    }
}
