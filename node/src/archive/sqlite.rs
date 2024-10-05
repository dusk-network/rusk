// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use anyhow::Result;
use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::Hash;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Pool, Sqlite,
};
use tracing::{error, info, warn};

use crate::archive::moonlight::MoonlightGroup;
use crate::archive::{Archive, Archivist};

/// The name of the archive SQLite database.
const SQLITEARCHIVE_DB_NAME: &str = "archive.sqlite3";

impl Archive {
    /// Create or open the SQLite database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the archive folder.
    pub(super) async fn create_or_open_sqlite<
        P: AsRef<Path> + std::fmt::Debug,
    >(
        path: P,
    ) -> Pool<Sqlite> {
        info!("Opening SQLite archive db in {path:?}");

        let db_options = SqliteConnectOptions::new()
            // append the database name to the path
            .filename(path.as_ref().join(SQLITEARCHIVE_DB_NAME))
            .create_if_missing(true);

        // Open the database, create it if it doesn't exist
        let archive_db = SqlitePool::connect_with(db_options)
            .await
            .expect("Failed to open archive database");

        // Run the migrations
        sqlx::migrate!("./migrations")
            .run(&archive_db)
            .await
            .expect("Failed to run migrations");

        archive_db
    }

    /// Fetch the json string of all vm events from a given block height
    pub async fn fetch_json_vm_events(
        &self,
        block_height: i64,
    ) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_height = ?"#,
            block_height
        ).fetch_one(&mut *conn).await?;

        Ok(events.json_contract_events)
    }

    /// Fetch the json string of all vm events from the last finalized block
    pub async fn fetch_json_last_vm_events(&self) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE finalized = 1 ORDER BY block_height DESC LIMIT 1"#
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok(events.json_contract_events)
    }

    /*
    todo: Implement fetch json last vm events where finalized = 0?
     */

    /// Fetch the json string of all vm events from a given block height
    pub async fn fetch_json_vm_events_by_blk_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok(events.json_contract_events)
    }

    /// Fetch all vm events from a given block hash
    pub async fn fetch_vm_events_by_blk_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<ContractTxEvent>> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok(serde_json::from_str(&events.json_contract_events)?)
    }
}

impl Archivist for Archive {
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
        let json_contract_events = serde_json::to_string(&events)?;

        let mut conn = self.sqlite_archive.acquire().await?;

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

        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_height = ?"#,
            block_height
        ).fetch_one(&mut *conn).await?;

        // convert the json string to a vector of ContractTxEvent and return it
        Ok(serde_json::from_str(&events.json_contract_events)?)
    }

    /// Mark the block of the given hash as finalized in the archive.
    ///
    /// This also triggers the loading of the MoonlightTxEvents into the
    /// moonlight db.
    async fn mark_block_finalized(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<()> {
        let current_block_height: i64 = current_block_height as i64;

        let mut conn = self.sqlite_archive.acquire().await?;

        // Set finalized to true for the block with the given hash
        // Return the block height
        let r = sqlx::query!(
            r#"UPDATE archive SET finalized = 1 WHERE block_hash = ?
            RETURNING block_height, json_contract_events
            "#,
            hex_block_hash
        )
        .fetch_one(&mut *conn)
        .await?;

        info!(
            "Marked block {} with height {} as finalized. After {} blocks at height {}",
            util::truncate_string(hex_block_hash),
            r.block_height,
            (current_block_height - r.block_height),
            current_block_height
        );

        if r.block_height < 0 {
            error!("Block height is negative. This is a bug.");
        }

        // Get the MoonlightTxEvents and load it into the moonlight db
        self.tl_moonlight(
            serde_json::from_str(&r.json_contract_events)?,
            r.block_height as u64,
        )?;

        Ok(())
    }

    /// Remove the block of the given hash from the archive.
    async fn remove_block(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<bool> {
        let block_height: i64 = current_block_height as i64;

        let mut conn = self.sqlite_archive.acquire().await?;

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
                util::truncate_string(hex_block_hash),
                r.block_height,
                block_height
            );
            Ok(true)
        } else {
            warn!(
                "Trying to delete Block {} which is finalized or does not exist in the archive",
                util::truncate_string(hex_block_hash)
            );
            Ok(false)
        }
    }

    /// Fetch the moonlight events for the given public key
    fn fetch_moonlight_histories(
        &self,
        address: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        // Get the moonlight events for the given public key from rocksdb
        self.full_moonlight_history(address)
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
    use super::*;
    use execution_core::ContractId;
    use node_data::events::contract::{ContractEvent, WrappedContractId};
    use rand::{distributions::Alphanumeric, Rng};
    use std::env;
    use std::path::PathBuf;
    use util::truncate_string;

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
        let path = test_dir();

        let archive = Archive::create_or_open(path).await;

        let events = dummy_data();

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
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;
        let blk_height = 1;
        let blk_hash = [5; 32];
        let hex_blk_hash = hex::encode(blk_hash);
        let events = dummy_data();

        archive
            .store_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        assert!(archive
            .remove_block(blk_height, &hex_blk_hash)
            .await
            .unwrap());

        let fetched_events = archive.fetch_vm_events(blk_height).await;

        assert!(fetched_events.is_err());

        archive
            .store_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        archive
            .mark_block_finalized(blk_height, &hex_blk_hash)
            .await
            .unwrap();

        assert!(!archive
            .remove_block(blk_height, &hex_blk_hash)
            .await
            .unwrap());
    }
}
