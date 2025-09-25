// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::Hash;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
    SqliteSynchronous,
};
use sqlx::{Pool, Sqlite, SqliteConnection};
use tracing::{error, info, warn};

use crate::archive::transformer;
use crate::archive::Archive;

/// The name of the archive SQLite database.
const SQLITEARCHIVE_DB_NAME: &str = "archive.sqlite3";

// The roles available for SQLite access.
#[derive(Clone, Copy)]
pub enum SqliteRole {
    Writer { max_connections: u32 },
    Reader { max_connections: u32 },
}

impl Archive {
    /// Build the base options (common to both reader and writer).
    fn base_connect_options<P: AsRef<Path>>(path: P) -> SqliteConnectOptions {
        SqliteConnectOptions::new()
            .filename(path.as_ref().join(SQLITEARCHIVE_DB_NAME))
            .pragma("trusted_schema", "OFF") // restrict potentially unsafe functions in schema objects
            .pragma("temp_store", "MEMORY") // keep temporary tables/indexes in RAM
            .pragma("mmap_size", "536870912") // enable memory-mapped I/O up to 512 MiB
            .pragma("cache_size", "-24576") // set page cache to 24 MiB
                                            // (negative value = KiB)
    }

    /// Create the single connection writer pool (WAL, FULL, foreign_keys,
    /// migrations).
    pub(super) async fn create_writer_pool<P: AsRef<Path> + std::fmt::Debug>(
        path: P,
    ) -> Pool<Sqlite> {
        Self::create_sqlite_pool(
            path,
            SqliteRole::Writer { max_connections: 1 },
        )
        .await
    }

    /// Create the read-only pool for GraphQL/queries (query_only).
    pub(super) async fn create_reader_pool<P: AsRef<Path> + std::fmt::Debug>(
        path: P,
    ) -> Pool<Sqlite> {
        Self::create_sqlite_pool(
            path,
            SqliteRole::Reader {
                max_connections: 16,
            },
        )
        .await
    }

    /// Create or open the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the archive folder.
    /// * `role` - Whether to open a writer (read-write) or reader (read-only)
    ///   pool.
    pub async fn create_sqlite_pool<P: AsRef<Path> + std::fmt::Debug>(
        path: P,
        role: SqliteRole,
    ) -> Pool<Sqlite> {
        let (opts, max_conns, run_migrations) = match role {
            SqliteRole::Writer { max_connections } => {
                // Writers need create_if_missing, WAL, FULL, FK, busy_timeout
                let opts = Self::base_connect_options(&path)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Full)
                    .busy_timeout(Duration::from_millis(5_000))
                    .pragma("foreign_keys", "ON");
                (opts, max_connections, true)
            }
            SqliteRole::Reader { max_connections } => {
                // Readers should be read-only, query_only
                let opts = Self::base_connect_options(&path)
                    .read_only(true)
                    .pragma("query_only", "ON");
                (opts, max_connections, false)
            }
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(max_conns)
            .connect_with(opts)
            .await
            .expect("Failed to open archive database");

        if run_migrations {
            // Only the writer runs migrations
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("Failed to run migrations");
        }

        pool
    }

    /// Fetch the json string of all vm events from a given block height
    pub async fn fetch_json_events_by_height(
        &self,
        block_height: i64,
    ) -> Result<String> {
        let events = self.fetch_events_by_height(block_height).await?;

        // Convert the event related row fields from finalized_events table to
        // json string
        Ok(serde_json::to_string(&events)?)
    }

    /// Fetch the list of all vm events from the block of the given height.
    async fn fetch_events_by_height(
        &self,
        block_height: i64,
    ) -> Result<Vec<data::ArchivedEvent>> {
        let mut conn = self.sqlite_reader.acquire().await?;

        // query all events now that we have the block height
        let records = sqlx::query_as!(data::ArchivedEvent,
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_height = ?
            UNION ALL
            SELECT origin, topic, source, data FROM finalized_events WHERE block_height = ? AND NOT EXISTS (SELECT 1 FROM unfinalized_events WHERE block_height = ?)
            "#,
            block_height, block_height, block_height
            ).fetch_all(&mut *conn).await?;

        Ok(records)
    }

    /// Fetch all vm events from a given block hash and return them as a json
    /// string
    pub async fn fetch_json_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<String> {
        let events = self.fetch_events_by_hash(hex_block_hash).await?;

        Ok(serde_json::to_string(&events)?)
    }

    /// Fetch all vm events from a given block hash
    async fn fetch_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<data::ArchivedEvent>> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let events = sqlx::query_as!(data::ArchivedEvent,
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_hash = ?
            UNION ALL
            SELECT origin, topic, source, data FROM finalized_events WHERE block_hash = ? AND NOT EXISTS (SELECT 1 FROM unfinalized_events WHERE block_hash = ?)
            "#,
            hex_block_hash, hex_block_hash, hex_block_hash
        ).fetch_all(&mut *conn).await?;

        Ok(events)
    }

    /// Fetch all vm events from the last block and return them as a
    /// json string
    pub async fn fetch_json_last_events(&self) -> Result<String> {
        let mut conn = self.sqlite_reader.acquire().await?;

        // Get the last finalized block height by getting all the events from
        // the largest block height
        let events = sqlx::query_as!(data::ArchivedEvent,
            r#"
                SELECT origin, topic, source, data FROM unfinalized_events
                WHERE block_height = (SELECT MAX(block_height) FROM unfinalized_events)
            "#
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(serde_json::to_string(&events)?)
    }

    /// Return a page of finalized events for a contract, ordered by `id`.
    ///
    /// # Arguments
    ///
    /// `limit` = max rows (server will clamp)
    /// `cursor` = return rows with id > cursor, None means from the beginning
    pub async fn fetch_finalized_events_from_contract(
        &self,
        contract_id: &str,
        limit: i64,
        cursor: Option<i64>,
    ) -> Result<(Vec<data::FinalizedEvent>, Option<i64>, bool)> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let effective_limit: i64 = limit.saturating_add(1);
        let cursor_val: Option<i64> = cursor;

        let mut rows = sqlx::query!(
            r#"SELECT id, block_height, block_hash, origin, topic, source, data
                FROM finalized_events
                WHERE source = ?1
                AND id > IFNULL(?2, -1)
                ORDER BY id ASC
                LIMIT ?3"#,
            contract_id,
            cursor_val,
            effective_limit
        )
        .fetch_all(&mut *conn)
        .await?;

        let has_next = (rows.len() as i64) > limit;
        if has_next {
            rows.truncate(limit as usize);
        }

        let next_cursor: Option<i64> = rows.last().map(|r| r.id);

        let events: Vec<data::FinalizedEvent> = rows
            .into_iter()
            .map(|r| data::FinalizedEvent {
                id: r.id,
                origin: r.origin,
                block_height: r.block_height,
                block_hash: r.block_hash,
                topic: r.topic,
                source: r.source,
                data: r.data,
            })
            .collect();

        Ok((events, next_cursor, has_next))
    }

    /// Fetch all unfinalized vm events for a block hash using an existing
    /// connection. This keeps finalize fully atomic and avoids mixing
    /// reader + writer pools.
    pub async fn fetch_unfinalized_events_by_hash<'t>(
        &self,
        conn: &mut SqliteConnection,
        hex_block_hash: &str,
    ) -> Result<Vec<ContractTxEvent>> {
        let unfinalized_events = sqlx::query_as!(data::ArchivedEvent,
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_all(conn)
        .await?;

        let mut contract_tx_events = Vec::new();
        for event in unfinalized_events {
            let contract_tx_event: ContractTxEvent = event.try_into()?;
            contract_tx_events.push(contract_tx_event);
        }

        Ok(contract_tx_events)
    }

    /// Fetch the last finalized block height and block hash
    pub async fn fetch_last_finalized_block(&self) -> Result<(u64, String)> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let block = sqlx::query!(
                r#"SELECT block_height, block_hash FROM finalized_blocks WHERE block_height = (SELECT MAX(block_height) FROM finalized_blocks)"#
            )
            .fetch_one(&mut *conn)
            .await?;

        Ok((block.block_height as u64, block.block_hash))
    }

    /// Check if a block_height & block_hash match a finalized block
    pub async fn match_finalized_block_height_hash(
        &self,
        block_height: i64,
        hex_block_hash: &str,
    ) -> Result<bool> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let r = sqlx::query!(
                    r#"SELECT block_height FROM finalized_blocks WHERE block_height = ? AND block_hash = ?"#,
                    block_height, hex_block_hash
                )
                .fetch_optional(&mut *conn)
                .await?;

        Ok(r.is_some())
    }

    /// Gives you the next block height that contains a phoenix event from a
    /// given starting block height
    pub async fn next_phoenix(&self, block_height: i64) -> Result<Option<u64>> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let r = sqlx::query!(
            r#"SELECT block_height FROM finalized_blocks WHERE block_height > ? AND phoenix_present = 1"#,
            block_height
        )
        .fetch_optional(&mut *conn)
        .await?;

        if let Some(record) = r {
            Ok(Some(record.block_height as u64))
        } else {
            Ok(None)
        }
    }

    pub async fn fetch_active_accounts(&self) -> Result<u64> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let last_account =
            sqlx::query!(r#"SELECT MAX(id) as last_id FROM active_accounts"#)
                .fetch_one(&mut *conn)
                .await?;
        let last_account_id = last_account.last_id.unwrap_or(0) as u64;

        Ok(last_account_id)
    }

    /// Count finalized transfer transactions for the transfer contract, split
    /// by topic. Returns (moonlight_count, phoenix_count).
    pub async fn fetch_tx_count(&self) -> Result<(u64, u64)> {
        let mut conn = self.sqlite_reader.acquire().await?;

        let transfer_src = dusk_core::transfer::TRANSFER_CONTRACT.to_string();

        let row = sqlx::query!(
            r#"
            SELECT
              CAST(COALESCE(SUM(CASE WHEN topic = 'moonlight' THEN 1 ELSE 0 END), 0) AS INTEGER) AS moonlight,
              CAST(COALESCE(SUM(CASE WHEN topic = 'phoenix'  THEN 1 ELSE 0 END), 0) AS INTEGER) AS phoenix
            FROM finalized_events
            WHERE source = ?
            "#,
            transfer_src
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok((row.moonlight as u64, row.phoenix as u64))
    }
}

/// Mutating methods for the SQLite Archive
impl Archive {
    /// Store the list of **all** unfinalized vm events from the block of the
    /// given height.
    pub(crate) async fn store_unfinalized_events(
        &self,
        block_height: u64,
        block_hash: Hash,
        events: Vec<ContractTxEvent>,
    ) -> Result<()> {
        let mut tx = self.sqlite_writer.begin().await?;

        let block_height: i64 = block_height as i64;
        let hex_block_hash = hex::encode(block_hash);

        sqlx::query!(
            r#"INSERT INTO unfinalized_blocks (block_height, block_hash) 
                VALUES (?, ?)
                ON CONFLICT(block_height) DO UPDATE SET block_hash = excluded.block_hash"#,
           block_height, hex_block_hash
       ).execute(&mut *tx).await?.rows_affected();

        // Convert the events to a data::Event
        for event in events {
            let event = data::ArchivedEvent {
                origin: hex::encode(event.origin),
                topic: event.event.topic,
                source: event.event.target.to_string(),
                data: event.event.data,
            };

            sqlx::query!(
                r#"INSERT INTO unfinalized_events (block_height, block_hash, origin, topic, source, data) VALUES (?, ?, ?, ?, ?, ?)"#,
                block_height, hex_block_hash, event.origin, event.topic, event.source, event.data
            )
            .execute(&mut *tx)
            .await?;
        }

        info!(
            "Archived unfinalized events from block {} with height {}",
            util::truncate_string(&hex_block_hash),
            block_height
        );

        tx.commit().await?;

        Ok(())
    }

    /// Finalize all data related to the block of the given hash in the archive.
    ///
    /// This also triggers the loading of the MoonlightTxEvents into the
    /// moonlight db. This also updates the last finalized block height
    /// attribute.
    pub(crate) async fn finalize_archive_data(
        &mut self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<()> {
        let mut tx = self.sqlite_writer.begin().await?;

        // Early-exit safeguard: Skip if this block is already finalized. Even
        // though inserts are idempotent, rerunning finalize would still
        // query/delete staging tables and reinsert events, adding
        // unnecessary locks. Transaction is deferred so this read takes
        // no write lock (yet).
        let already_finalized = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                SELECT 1 
                FROM finalized_blocks 
                WHERE block_hash = ?
            ) AS "exists!: bool""#,
            hex_block_hash
        )
        .fetch_one(&mut *tx)
        .await?;

        if already_finalized {
            warn!(
                "archive: finalize called for an already finalized block: {}",
                util::truncate_string(hex_block_hash)
            );
            // No changes have been made, explicitly rollback immediately
            tx.rollback().await?;
            return Ok(());
        }

        // Get the row for the block with the given hash that got finalized
        let r = sqlx::query!(
            r#"SELECT * FROM unfinalized_blocks WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(r) = r else {
            warn!(
                "archive: finalize called but no unfinalized row found for block: {}",
                util::truncate_string(hex_block_hash)
            );
            tx.rollback().await?;
            return Ok(());
        };

        let finalized_block_height = r.block_height;
        if finalized_block_height < 0 {
            error!("Block height is negative. This is a bug.");
        }

        // Get all the unfinalized events from the block
        let events = self
            .fetch_unfinalized_events_by_hash(&mut tx, hex_block_hash)
            .await?;

        /*
        Cases of phoenix transfers that produced `Notes` as output:
        1. Any PhoenixTransactionEvent (through notes & refund_note)
        */
        let phoenix_event_present = events.iter().any(|event| {
            event.event.target == dusk_core::transfer::TRANSFER_CONTRACT
                && event.event.topic == dusk_core::transfer::PHOENIX_TOPIC
        });

        // Group events by origin (block height, OriginHash)
        let grouped_events = transformer::group_by_origins(
            events,
            finalized_block_height as u64,
        );

        // TODO: We can categorize grouped_events at one point here too and add
        // this data to another table

        let existed = sqlx::query_scalar!(
            r#"SELECT 1 FROM finalized_blocks WHERE id = ? LIMIT 1"#,
            finalized_block_height
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        let affected = sqlx::query!(
            r#"INSERT INTO finalized_blocks (id, block_height, block_hash, phoenix_present)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(block_height) DO UPDATE SET
                    block_hash = excluded.block_hash,
                    phoenix_present = excluded.phoenix_present"#,
            finalized_block_height, finalized_block_height, hex_block_hash, phoenix_event_present
        ).execute(&mut *tx).await?.rows_affected();

        if existed && affected == 1 {
            error!(
                "archive: finalized_blocks upsert used DO UPDATE (unexpected path) for height {} hash {}",
                finalized_block_height,
                hex_block_hash
            );
        }

        sqlx::query!(
            r#"DELETE FROM unfinalized_events WHERE block_hash = ?"#,
            hex_block_hash
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"DELETE FROM unfinalized_blocks WHERE block_hash = ?"#,
            hex_block_hash
        )
        .execute(&mut *tx)
        .await?;

        // Get all ContractEvents and insert them into the appropriate
        // finalized tables
        for (ident, events) in &grouped_events {
            let origin = hex::encode(ident.origin());

            for event in events {
                let source = event.target.to_string();

                sqlx::query!(
                    r#"INSERT INTO finalized_events (block_height, block_hash, origin, topic, source, data) VALUES (?, ?, ?, ?, ?, ?)"#,
                    finalized_block_height, hex_block_hash, origin, event.topic, source, event.data
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        let current_block_height: i64 = current_block_height as i64;
        info!(
            "Marked block {} with height {} as finalized. After {} blocks at height {}",
            util::truncate_string(hex_block_hash),
            finalized_block_height,
            (current_block_height - finalized_block_height),
            current_block_height
        );

        // Get the MoonlightTxEvents and load it into the moonlight db
        let active_accounts = self.tl_moonlight(grouped_events)?;

        self.update_active_accounts(active_accounts).await?;

        self.last_finalized_block_height = finalized_block_height as u64;

        Ok(())
    }

    /// Remove the unfinalized block together with the unfinalized events of the
    /// given hash from the archive.
    pub(crate) async fn remove_block_and_events(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<bool> {
        let block_height: i64 = current_block_height as i64;

        let mut tx = self.sqlite_writer.begin().await?;

        sqlx::query!(
            r#"DELETE FROM unfinalized_events WHERE block_hash = ?"#,
            hex_block_hash
        )
        .execute(&mut *tx)
        .await?;

        let r = sqlx::query!(
            r#"DELETE FROM unfinalized_blocks WHERE block_hash = ?
            RETURNING block_height
            "#,
            hex_block_hash
        )
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Some(r) = r {
            info!(
                "Deleted unfinalized events from block {} with block height: {} at height {}",
                util::truncate_string(hex_block_hash),
                r.block_height,
                block_height
            );
            Ok(true)
        } else {
            warn!(
                "Trying to delete unfinalized block {} which does not exist in the archive",
                util::truncate_string(hex_block_hash)
            );
            Ok(false)
        }
    }

    /// Insert the active accounts into the active accounts table, skipping
    /// already existing ones
    ///
    /// TODO: This function should not be public.
    pub async fn update_active_accounts(
        &self,
        active_accounts: HashSet<String>,
    ) -> Result<u64> {
        let mut tx = self.sqlite_writer.begin().await?;

        for account in active_accounts {
            sqlx::query!(
                r#"INSERT OR IGNORE INTO active_accounts (public_key) VALUES (?)"#,
                account
            )
            .execute(&mut *tx)
            .await?;
        }

        let last_account =
            sqlx::query!(r#"SELECT MAX(id) as last_id FROM active_accounts"#)
                .fetch_one(&mut *tx)
                .await?;

        let last_account_id = last_account.last_id.unwrap_or(0) as u64;

        tx.commit().await?;

        info!(
            "Updated active accounts in the archive. Last account ID: {}",
            last_account_id
        );

        Ok(last_account_id)
    }
}

mod data {
    use node_data::events::contract::{
        ContractEvent, ContractTxEvent, ORIGIN_HASH_BYTES,
    };
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;

    /// Data transfer object for GraphQL pagination
    #[serde_with::serde_as]
    #[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
    pub struct FinalizedEvent {
        pub id: i64,
        pub block_height: i64,
        pub block_hash: String,
        pub origin: String,
        pub topic: String,
        pub source: String,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub data: Vec<u8>,
    }

    /// Archived ContractTxEvent
    ///
    /// This struct is used to store the archived events in the SQLite database.
    ///
    /// # Fields
    /// - `origin`: The origin field is the hex encoded origin hash of the
    ///   event.
    /// - `topic`: The topic field is the topic of the event.
    /// - `source`: The source field is the hex encoded contract id of the
    ///   event.
    /// - `data`: The data field is the data of the event.
    #[serde_with::serde_as]
    #[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
    pub struct ArchivedEvent {
        pub origin: String,
        pub topic: String,
        pub source: String,
        #[serde_as(as = "serde_with::hex::Hex")]
        pub data: Vec<u8>,
    }

    impl TryFrom<ArchivedEvent> for ContractTxEvent {
        type Error = anyhow::Error;

        fn try_from(value: ArchivedEvent) -> Result<Self, Self::Error> {
            let origin = hex::decode(&value.origin)?;
            let mut origin_array = [0u8; 32];

            // convert Vec<u8> to [u8; 32]
            if origin.len() != ORIGIN_HASH_BYTES {
                return Err(anyhow::anyhow!(
                    "Invalid length: expected 32 bytes, got {}",
                    origin.len()
                ));
            } else {
                origin_array.copy_from_slice(&origin);
            }

            let target = value.source.try_into()?;

            Ok(ContractTxEvent {
                event: ContractEvent {
                    target,
                    topic: value.topic,
                    data: value.data,
                },
                origin: origin_array,
            })
        }
    }
}

mod util {
    /// Truncate a string to at most 35 characters.
    pub(super) fn truncate_string(s: &str) -> String {
        if s.len() <= 35 {
            return s.to_string();
        }

        s.chars().take(16).collect::<String>()
            + "..."
            + &s.chars()
                .rev()
                .take(16)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use dusk_core::abi::ContractId;
    use node_data::events::contract::ContractEvent;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use util::truncate_string;

    use super::*;

    #[test]
    fn test_truncate_string() {
        let s = "0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            util::truncate_string(s),
            "0123456789abcdef...0123456789abcdef"
        );

        let s = "1";
        assert_eq!(util::truncate_string(s), "1");

        let mut s = String::new();
        truncate_string(&s);

        for _ in 0..100 {
            s.push_str(&"0");
            util::truncate_string(&s);
        }
    }

    // Construct a random test directory path in the temp folder of the OS
    fn test_dir() -> PathBuf {
        let mut test_dir = "archive-sqlite-db-test-".to_owned();
        let rand_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        test_dir.push_str(&rand_string);

        env::temp_dir().join(test_dir)
    }

    fn dummy_data() -> Vec<ContractTxEvent> {
        vec![
            ContractTxEvent {
                event: ContractEvent {
                    target: ContractId::from_bytes([0; 32]),
                    topic: "contract1".to_string(),
                    data: vec![1, 6, 1, 8],
                },
                origin: [0; 32],
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: ContractId::from_bytes([1; 32]),
                    topic: "contract2".to_string(),
                    data: vec![1, 2, 3],
                },
                origin: [1; 32],
            },
        ]
    }

    #[tokio::test]
    async fn test_store_fetch_vm_events() {
        let path = test_dir();

        let archive = Archive::create_or_open(path).await;

        let events = dummy_data();

        archive
            .store_unfinalized_events(1, [5; 32], events.clone())
            .await
            .unwrap();

        let fetched_events = archive.fetch_events_by_height(1).await.unwrap();

        // Check if the events are the same
        for (contract_tx_event, fetched_event) in
            events.iter().zip(fetched_events.iter())
        {
            assert_eq!(
                contract_tx_event.event.target.to_string(),
                fetched_event.source /* if this fails do hex::decode here
                                      * and to bytes above */
            );
            assert_eq!(contract_tx_event.event.topic, fetched_event.topic);
            assert_eq!(contract_tx_event.event.data, fetched_event.data);
            assert_eq!(
                contract_tx_event.origin,
                &hex::decode(&fetched_event.origin).unwrap()[..]
            );
        }
    }

    #[tokio::test]
    async fn test_delete_vm_events() {
        let path = test_dir();
        let mut archive = Archive::create_or_open(path).await;
        let blk_height = 1;
        let blk_hash = [5; 32];
        let hex_blk_hash = hex::encode(blk_hash);
        let events = dummy_data();

        archive
            .store_unfinalized_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        assert!(archive
            .remove_block_and_events(blk_height, &hex_blk_hash)
            .await
            .unwrap());

        let fetched_events = archive
            .fetch_events_by_height(blk_height as i64)
            .await
            .unwrap();
        assert!(fetched_events.is_empty());

        archive
            .store_unfinalized_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        archive
            .finalize_archive_data(blk_height, &hex_blk_hash)
            .await
            .unwrap();

        assert!(!archive
            .remove_block_and_events(blk_height, &hex_blk_hash)
            .await
            .unwrap());

        let (blk_height, blk_hash) =
            archive.fetch_last_finalized_block().await.unwrap();
        assert_eq!(blk_height, 1);
        assert_eq!(blk_hash, hex_blk_hash);
    }
}
