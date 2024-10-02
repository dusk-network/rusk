// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::Hash;
use rocksdb::OptimisticTransactionDB;
use sqlx::sqlite::SqlitePool;

mod archivist;
mod moonlight;
mod sqlite;
mod transformer;

pub use archivist::ArchivistSrv;
pub use transformer::MoonlightTxEvents;

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
            Self::create_or_open_moonlight_db(&path, ArchiveOptions::default())
                .await;

        Self {
            sqlite_archive,
            moonlight_db,
        }
    }
}

/// The Archivist is responsible for storing the events and potentially other
/// ephemeral data forever in the archive DB.
///
/// Example:
/// - The block does not store the events, only the hash of the events. The
///   archivist will store the events.
pub(crate) trait Archivist {
    /// Store the list of all vm events from the block of the given height &
    /// hash.
    async fn store_vm_events(
        &self,
        block_height: u64,
        block_hash: Hash,
        events: Vec<ContractTxEvent>,
    ) -> Result<()>;

    /// Fetch the list of all vm events from the block of the given height.
    async fn fetch_vm_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<ContractTxEvent>>;

    /// Fetch the moonlight events for the given public key
    fn fetch_moonlight_histories(
        &self,
        address: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightTxEvents>>>;

    async fn mark_block_finalized(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<()>;

    async fn remove_block(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<bool>;
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
