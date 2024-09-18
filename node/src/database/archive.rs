// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::Path;

use anyhow::Result;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::Hash;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use tracing::{info, warn};

use super::Archivist;

const ARCHIVE_FOLDER_NAME: &str = "archive";
const SQLITE_DB_NAME: &str = "archive.sqlite3";

#[derive(Debug)]
pub struct SQLiteArchive {
    archive_db: SqlitePool,
}

impl SQLiteArchive {
    pub async fn create_or_open<T>(path: T) -> Self
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().join(ARCHIVE_FOLDER_NAME);
        info!("Opening archive db in {path:?}");

        // Recursively create the archive folder if it doesn't exist already
        fs::create_dir_all(&path)
            .expect("creating directory in {path} should not fail");

        let db_options = SqliteConnectOptions::new()
            // append the database name to the path
            .filename(path.join(SQLITE_DB_NAME))
            .create_if_missing(true);

        // Open the database, create it if it doesn't exist
        let archive_db = SqlitePool::connect_with(db_options)
            .await
            .expect("Failed to open database");

        // Run the migrations
        sqlx::migrate!("./migrations")
            .run(&archive_db)
            .await
            .expect("Failed to run migrations");

        Self { archive_db }
    }

    /// Fetch the json string of all vm events form a given block height
    pub async fn fetch_json_vm_events(
        &self,
        block_height: u64,
    ) -> Result<String> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.archive_db.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_height = ?"#,
            block_height
        ).fetch_one(&mut *conn).await?;

        Ok(events.json_contract_events)
    }
}

impl Archivist for SQLiteArchive {
    /// Store the list of all vm events from the block of the given height.
    async fn store_vm_events(
        &self,
        block_height: u64,
        block_hash: Hash,
        events: Vec<ContractTxEvent>,
    ) -> Result<()> {
        let block_height: i64 = block_height as i64;
        let hex_block_hash = hex::encode(block_hash);
        // Serialize the events to a json string
        let json_contract_events = serde_json::to_string(&events).unwrap();

        let mut conn = self.archive_db.acquire().await?;

        sqlx::query!(
             r#"INSERT INTO archive (block_height, block_hash, json_contract_events) VALUES (?, ?, ?)"#,
            block_height, hex_block_hash, json_contract_events
        ).execute(&mut *conn).await?.rows_affected();

        info!(
            "Archived events from block {} with height {}",
            util::truncate_string(&hex_block_hash),
            block_height
        );

        Ok(())
    }

    /// Fetch the list of all vm events from the block of the given height.
    async fn fetch_vm_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<ContractTxEvent>> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.archive_db.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_height = ?"#,
            block_height
        ).fetch_one(&mut *conn).await?;

        // convert the json string to a vector of ContractTxEvent and return it
        Ok(serde_json::from_str(&events.json_contract_events)?)
    }

    /// Mark the block of the given height and hash as finalized in the archive.
    async fn mark_block_finalized(
        &self,
        block_height: u64,
        hex_block_hash: String,
    ) -> Result<()> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.archive_db.acquire().await?;

        // Set finalized to true for the block with the given hash
        // Return the block height
        let r = sqlx::query!(
            r#"UPDATE archive SET finalized = 1 WHERE block_hash = ?
            RETURNING block_height
            "#,
            hex_block_hash
        )
        .fetch_one(&mut *conn)
        .await?;

        info!(
            "Marked block {} as finalized at height {}. After {} blocks",
            util::truncate_string(&hex_block_hash),
            block_height,
            block_height - r.block_height
        );

        Ok(())
    }

    /// Remove the block of the given height and hash from the archive.
    async fn remove_deleted_block(
        &self,
        block_height: u64,
        hex_block_hash: String,
    ) -> Result<bool> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.archive_db.acquire().await?;

        let r = sqlx::query!(
            r#"DELETE FROM archive WHERE block_hash = ? AND (finalized IS NULL OR finalized = 0)
            RETURNING block_height
            "#,
            hex_block_hash
        )
        .fetch_optional(&mut *conn)
        .await?;

        if let Some(r) = r {
            info!(
                "Deleted events from block {} with block height: {} at height {}",
                util::truncate_string(&hex_block_hash),
                r.block_height,
                block_height
            );
            Ok(true)
        } else {
            warn!(
                "Trying to delete Block {} which is finalized or does not exist in the archive",
                util::truncate_string(&hex_block_hash)
            );
            Ok(false)
        }
    }
}

mod util {
    /// Truncate a string to at most 35 characters.
    pub fn truncate_string(s: &str) -> String {
        if s.len() <= 32 {
            return s.to_string();
        }

        let first_part = &s[..16];
        let last_part = &s[s.len() - 16..];
        format!("{}...{}", first_part, last_part)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution_core::ContractId;
    use node_data::events::contract::{ContractEvent, WrappedContractId};
    use rand::{distributions::Alphanumeric, Rng};
    use std::env;
    use std::path::PathBuf;

    // Construct a random test directory path in the temp folder of the OS
    fn get_test_dir() -> PathBuf {
        let mut test_dir = "archive-db-test-".to_owned();
        let rand_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        test_dir.push_str(&rand_string);

        env::temp_dir().join(test_dir)
    }

    fn get_dummy_data() -> Vec<ContractTxEvent> {
        vec![
            ContractTxEvent {
                event: ContractEvent {
                    target: WrappedContractId(ContractId::from_bytes([0; 32])),
                    topic: "contract1".to_string(),
                    data: vec![1, 6, 1, 8],
                },
                origin: Some([0; 32]),
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: WrappedContractId(ContractId::from_bytes([1; 32])),
                    topic: "contract2".to_string(),
                    data: vec![1, 2, 3],
                },
                origin: Some([1; 32]),
            },
        ]
    }

    #[tokio::test]
    async fn test_store_fetch_vm_events() {
        let path = get_test_dir();

        let archive = SQLiteArchive::create_or_open(path).await;

        let events = get_dummy_data();

        archive
            .store_vm_events(1, [5; 32], events.clone())
            .await
            .unwrap();

        let fetched_events = archive.fetch_vm_events(1).await.unwrap();

        // Check if the events are the same
        for (contract_tx_event, fetched_event) in
            events.iter().zip(fetched_events.iter())
        {
            assert_eq!(
                contract_tx_event.event.target,
                fetched_event.event.target
            );
            assert_eq!(
                contract_tx_event.event.topic,
                fetched_event.event.topic
            );
            assert_eq!(contract_tx_event.event.data, fetched_event.event.data);
            assert_eq!(contract_tx_event.origin, fetched_event.origin);
        }
    }

    #[tokio::test]
    async fn test_delete_vm_events() {
        let path = get_test_dir();
        let archive = SQLiteArchive::create_or_open(path).await;
        let blk_height = 1;
        let blk_hash = [5; 32];
        let hex_blk_hash = hex::encode(blk_hash);
        let events = get_dummy_data();

        archive
            .store_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        assert!(archive
            .remove_deleted_block(blk_height, hex_blk_hash.clone())
            .await
            .unwrap());

        let fetched_events = archive.fetch_vm_events(blk_height).await;

        assert!(fetched_events.is_err());

        archive
            .store_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        archive
            .mark_block_finalized(blk_height, hex_blk_hash.clone())
            .await
            .unwrap();

        assert!(!archive
            .remove_deleted_block(blk_height, hex_blk_hash)
            .await
            .unwrap());
    }
}
