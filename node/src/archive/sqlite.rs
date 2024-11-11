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
use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Pool, Sqlite,
};
use tracing::{error, info, warn};

use crate::archive::moonlight::MoonlightGroup;
use crate::archive::transformer;
use crate::archive::Archive;

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
    pub async fn fetch_json_vm_events_by_blk_height(
        &self,
        block_height: i64,
    ) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_height = ?
            UNION ALL
            SELECT origin, topic, source, data FROM finalized_events WHERE block_height = ? AND NOT EXISTS (SELECT 1 FROM unfinalized_events WHERE block_height = ?)
            "#,
            block_height, block_height, block_height
        ).fetch_all(&mut *conn).await?;

        // Convert the event related row fields from finalized_events table to
        // json string
        let json = json!(events
            .into_iter()
            .map(|record| {
                json!({
                    "origin": record.origin,
                    "topic": record.topic,
                    "source": record.source,
                    "data": record.data,
                })
            })
            .collect::<Vec<_>>());

        Ok(json.to_string())
    }

    /// Fetch all vm events from the last finalized block and return them as a
    /// json string
    pub async fn fetch_json_last_vm_events(&self) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        // Get the last finalized block height by getting all the events from
        // the largest block height
        let events = sqlx::query!(
            r#"
                SELECT origin, topic, source, data FROM finalized_events
                WHERE block_height = (SELECT MAX(block_height) FROM finalized_events)
            "#
        )
        .fetch_all(&mut *conn)
        .await?;

        // Convert the event related row fields from finalized_events table to
        // json string
        let json = json!(events
            .into_iter()
            .map(|record| {
                json!({
                    "origin": record.origin,
                    "topic": record.topic,
                    "source": record.source,
                    "data": record.data,
                })
            })
            .collect::<Vec<_>>());

        Ok(json.to_string())
    }

    /// Fetch all vm events from a given block hash and return them as a json
    /// string
    pub async fn fetch_json_vm_events_by_blk_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<String> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let events = sqlx::query!(
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_hash = ?
            UNION ALL
            SELECT origin, topic, source, data FROM finalized_events WHERE block_hash = ? AND NOT EXISTS (SELECT 1 FROM unfinalized_events WHERE block_hash = ?)
            "#,
            hex_block_hash, hex_block_hash, hex_block_hash
        ).fetch_all(&mut *conn).await?;

        // Convert the event related row fields from finalized_events table to
        // json string
        let json = json!(events
            .into_iter()
            .map(|row| {
                json!({
                    "origin": row.origin,
                    "topic": row.topic,
                    "source": row.source,
                    "data": row.data,
                })
            })
            .collect::<Vec<_>>());

        Ok(json.to_string())
    }

    /// Fetch all vm events from a given block hash
    pub async fn fetch_vm_events_by_blk_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<ContractTxEvent>> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let records = sqlx::query!(
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_hash = ?
            UNION ALL
            SELECT origin, topic, source, data FROM finalized_events WHERE block_hash = ? AND NOT EXISTS (SELECT 1 FROM unfinalized_events WHERE block_hash = ?)
            "#,
            hex_block_hash, hex_block_hash, hex_block_hash
        ).fetch_all(&mut *conn).await?;

        // Convert the event related row fields from finalized_events table to
        // data::Events and then to ContractTxEvent through into()
        let mut contract_tx_events = Vec::new();
        for row in records {
            let event = data::ArchivedEvent {
                origin: row.origin,
                topic: row.topic,
                source: row.source,
                data: row.data,
            };
            let contract_tx_event: ContractTxEvent = event.try_into()?;
            contract_tx_events.push(contract_tx_event);
        }

        Ok(contract_tx_events)
    }

    /// Fetch the list of all vm events from the block of the given height.
    pub async fn fetch_vm_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<data::ArchivedEvent>> {
        let block_height: i64 = block_height as i64;

        let mut conn = self.sqlite_archive.acquire().await?;

        // query all events now that we have the block height
        let records = sqlx::query!(
                r#"SELECT origin, topic, source, data FROM finalized_events WHERE block_height = ?"#,
                block_height
            ).fetch_all(&mut *conn).await?;

        Ok(records
            .into_iter()
            .map(|record| data::ArchivedEvent {
                origin: record.origin,
                topic: record.topic,
                source: record.source,
                data: record.data,
            })
            .collect())
    }

    /// Fetch the last finalized block height and block hash
    pub async fn fetch_last_finalized_block(&self) -> Result<(u64, String)> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let block = sqlx::query!(
            r#"SELECT block_height, block_hash FROM finalized_blocks WHERE block_height = (SELECT MAX(block_height) FROM finalized_blocks)"#
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok((block.block_height as u64, block.block_hash))
    }

    /// Get all finalized events from a specific contract
    pub async fn fetch_finalized_events_from_contract(
        &self,
        contract_id: &str,
    ) -> Result<Vec<data::ArchivedEvent>> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let records = sqlx::query!(
            r#"SELECT origin, topic, source, data FROM finalized_events WHERE source = ?"#,
            contract_id
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(records
            .into_iter()
            .map(|record| data::ArchivedEvent {
                origin: record.origin,
                topic: record.topic,
                source: record.source,
                data: record.data,
            })
            .collect())
    }

    /// Fetch all unfinalized vm events from a given block hash
    pub async fn fetch_unfinalized_vm_events_by_blk_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<ContractTxEvent>> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let records = sqlx::query!(
            r#"SELECT origin, topic, source, data FROM unfinalized_events WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_all(&mut *conn)
        .await?;

        let mut contract_tx_events = Vec::new();
        for row in records {
            let event = data::ArchivedEvent {
                origin: row.origin,
                topic: row.topic,
                source: row.source,
                data: row.data,
            };
            let contract_tx_event: ContractTxEvent = event.try_into()?;
            contract_tx_events.push(contract_tx_event);
        }

        Ok(contract_tx_events)
    }

    /// Fetch the finalized moonlight events for the given public key
    pub fn fetch_moonlight_histories(
        &self,
        address: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        // Get the moonlight events for the given public key from rocksdb
        self.full_moonlight_history(address)
    }

    /// Check if a block_height & block_hash match a finalized block
    pub async fn match_finalized_block_height_hash(
        &self,
        block_height: i64,
        hex_block_hash: &str,
    ) -> Result<bool> {
        let mut conn = self.sqlite_archive.acquire().await?;

        let r = sqlx::query!(
            r#"SELECT block_height FROM finalized_blocks WHERE block_height = ? AND block_hash = ?"#,
            block_height, hex_block_hash
        )
        .fetch_optional(&mut *conn)
        .await?;

        Ok(r.is_some())
    }
}

/// Mutating methods for the SQLite Archive
impl Archive {
    /// Store the list of **all** unfinalized vm events from the block of the
    /// given height.
    pub(super) async fn store_unfinalized_vm_events(
        &self,
        block_height: u64,
        block_hash: Hash,
        events: Vec<ContractTxEvent>,
    ) -> Result<()> {
        let mut tx = self.sqlite_archive.begin().await?;

        let block_height: i64 = block_height as i64;
        let hex_block_hash = hex::encode(block_hash);

        sqlx::query!(
            r#"INSERT INTO unfinalized_blocks (block_height, block_hash) VALUES (?, ?)"#,
           block_height, hex_block_hash
       ).execute(&mut *tx).await?.rows_affected();

        // Convert the events to a data::Event
        for event in events {
            let event = data::ArchivedEvent {
                origin: hex::encode(event.origin),
                topic: event.event.topic,
                source: event.event.target.0.to_string(),
                data: event.event.data,
            };

            let mut conn = self.sqlite_archive.acquire().await?;

            sqlx::query!(
                r#"INSERT INTO unfinalized_events (block_height, block_hash, origin, topic, source, data) VALUES (?, ?, ?, ?, ?, ?)"#,
                block_height, hex_block_hash, event.origin, event.topic, event.source, event.data
            )
            .execute(&mut *conn)
            .await?;
        }

        info!(
            "Archived unfinalized events from block {} with height {}",
            util::truncate_string(&hex_block_hash),
            block_height
        );

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Finalize all data related to the block of the given hash in the archive.
    ///
    /// This also triggers the loading of the MoonlightTxEvents into the
    /// moonlight db.
    pub(super) async fn finalize_archive_data(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<()> {
        let current_block_height: i64 = current_block_height as i64;

        let mut tx = self.sqlite_archive.begin().await?;

        // Get the row for the block with the given hash that got finalized
        let r = sqlx::query!(
            r#"SELECT * FROM unfinalized_blocks WHERE block_hash = ?"#,
            hex_block_hash
        )
        .fetch_one(&mut *tx)
        .await?;
        let finalized_block_height = r.block_height;
        if finalized_block_height < 0 {
            error!("Block height is negative. This is a bug.");
        }

        // Get all the unfinalized events from the block
        let events = self
            .fetch_unfinalized_vm_events_by_blk_hash(hex_block_hash)
            .await?;

        // Group events by origin (block height, TxHash) & throw away the ones
        // that don't have an origin
        let grouped_events = transformer::group_by_origins(
            events,
            finalized_block_height as u64,
        );

        // TODO Categorize events?

        sqlx::query!(
            r#"INSERT INTO finalized_blocks (block_height, block_hash) VALUES (?, ?)"#,
            finalized_block_height, hex_block_hash
        ).execute(&mut *tx).await?;

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
                // TODO: make conversion easier or remove it and insert directly
                // into query
                let event = data::ArchivedEvent {
                    origin: origin.clone(),
                    topic: event.topic.clone(),
                    source: event.target.0.to_string(),
                    data: event.data.clone(),
                };

                sqlx::query!(
                    r#"INSERT INTO finalized_events (block_height, block_hash, origin, topic, source, data) VALUES (?, ?, ?, ?, ?, ?)"#,
                    finalized_block_height, hex_block_hash, event.origin, event.topic, event.source, event.data
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        // Commit the transaction
        tx.commit().await?;

        info!(
            "Marked block {} with height {} as finalized. After {} blocks at height {}",
            util::truncate_string(hex_block_hash),
            finalized_block_height,
            (current_block_height - finalized_block_height),
            current_block_height
        );

        // Get the MoonlightTxEvents and load it into the moonlight db
        self.tl_moonlight(grouped_events)?;

        Ok(())
    }

    /// Remove the block of the given hash from the archive.
    pub(super) async fn remove_block(
        &self,
        current_block_height: u64,
        hex_block_hash: &str,
    ) -> Result<bool> {
        let block_height: i64 = current_block_height as i64;

        let mut tx = self.sqlite_archive.begin().await?;

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
}

mod data {
    use node_data::events::contract::{
        ContractEvent, ContractTxEvent, ORIGIN_HASH_BYTES,
    };

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
    pub struct ArchivedEvent {
        pub origin: String,
        pub topic: String,
        pub source: String,
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

            Ok(ContractTxEvent {
                event: ContractEvent {
                    target: value.source.try_into()?,
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
                origin: [0; 32],
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: WrappedContractId(ContractId::from_bytes([1; 32])),
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
            .store_unfinalized_vm_events(1, [5; 32], events.clone())
            .await
            .unwrap();

        let fetched_events = archive.fetch_vm_events(1).await.unwrap();

        // Check if the events are the same
        for (contract_tx_event, fetched_event) in
            events.iter().zip(fetched_events.iter())
        {
            assert_eq!(
                contract_tx_event.event.target.0.to_string(),
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
        let archive = Archive::create_or_open(path).await;
        let blk_height = 1;
        let blk_hash = [5; 32];
        let hex_blk_hash = hex::encode(blk_hash);
        let events = dummy_data();

        archive
            .store_unfinalized_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        assert!(archive
            .remove_block(blk_height, &hex_blk_hash)
            .await
            .unwrap());

        let fetched_events = archive.fetch_vm_events(blk_height).await;

        assert!(fetched_events.is_err());

        archive
            .store_unfinalized_vm_events(blk_height, blk_hash, events.clone())
            .await
            .unwrap();

        archive
            .finalize_archive_data(blk_height, &hex_blk_hash)
            .await
            .unwrap();

        assert!(!archive
            .remove_block(blk_height, &hex_blk_hash)
            .await
            .unwrap());

        let (blk_height, blk_hash) =
            archive.fetch_last_finalized_block().await.unwrap();
        assert_eq!(blk_height, 1);
        assert_eq!(blk_hash, hex_blk_hash);
    }
}
