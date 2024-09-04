// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::Path;

use anyhow::Result;
use node_data::archive::ContractEvent;
use node_data::ledger::Hash;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use tracing::info;

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
        events: Vec<ContractEvent>,
    ) -> Result<()> {
        let block_height: i64 = block_height as i64;
        let block_hash = hex::encode(block_hash);
        // Serialize the events to a json string
        let json_contract_events = serde_json::to_string(&events).unwrap();

        let mut conn = self.archive_db.acquire().await?;

        sqlx::query!(
             r#"INSERT INTO archive (block_height, block_hash, json_contract_events) VALUES (?, ?, ?)"#,
            block_height, block_hash, json_contract_events
        ).execute(&mut *conn).await?.rows_affected();

        info!("Stored events in block: {}", block_height);

        Ok(())
    }

    async fn fetch_vm_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<ContractEvent>> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.archive_db.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT json_contract_events FROM archive WHERE block_height = ?"#,
            block_height
        ).fetch_one(&mut *conn).await?;

        // convert the json string to a vector of ContractEvent and return it
        Ok(serde_json::from_str(&events.json_contract_events)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{distributions::Alphanumeric, Rng};
    use std::env;
    use std::path::{self, PathBuf};

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

    #[tokio::test]
    async fn test_store_fetch_vm_events() {
        let path = get_test_dir();

        let archive = SQLiteArchive::create_or_open(path).await;

        let events = vec![
            ContractEvent {
                source: [0; 32],
                topic: "contract1".to_string(),
                data: vec![1, 6, 1, 8],
            },
            ContractEvent {
                source: [0; 32],
                topic: "contract2".to_string(),
                data: vec![1, 2, 3],
            },
        ];

        archive
            .store_vm_events(1, [5; 32], events.clone())
            .await
            .unwrap();

        let fetched_events = archive.fetch_vm_events(1).await.unwrap();

        // Check if the events are the same
        for (event, fetched_event) in events.iter().zip(fetched_events.iter()) {
            assert_eq!(event.source, fetched_event.source);
            assert_eq!(event.topic, fetched_event.topic);
            assert_eq!(event.data, fetched_event.data);
        }
    }
}
