// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Integration tests for the Archive ETL pipeline system.
//!
//! These tests validate end-to-end functionality including finalization,
//! filtering, checkpoint management, and data integrity.

#[cfg(test)]
mod tests {
    use super::super::conf::Params as ArchiveParams;
    use super::super::Archive;
    use dusk_core::abi::ContractId;
    use node_data::events::contract::{ContractEvent, ContractTxEvent};
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_dir(prefix: &str) -> std::path::PathBuf {
        use rand::distributions::Alphanumeric;
        use rand::Rng;

        let rand_suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        env::temp_dir().join(format!("{}-{}", prefix, rand_suffix))
    }

    fn create_config_file(config_json: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_json.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    /// Standard test events: 2 moonlight (transfer), 1 phoenix (transfer), 1
    /// other_topic (different contract)
    fn dummy_events() -> Vec<ContractTxEvent> {
        let transfer = dusk_core::transfer::TRANSFER_CONTRACT;
        vec![
            ContractTxEvent {
                event: ContractEvent {
                    target: transfer,
                    topic: "moonlight".to_string(),
                    data: vec![1, 2, 3, 4],
                },
                origin: [1; 32],
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: transfer,
                    topic: "phoenix".to_string(),
                    data: vec![5, 6, 7, 8],
                },
                origin: [1; 32],
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: transfer,
                    topic: "moonlight".to_string(),
                    data: vec![9, 10, 11, 12],
                },
                origin: [2; 32],
            },
            ContractTxEvent {
                event: ContractEvent {
                    target: ContractId::from_bytes([99; 32]),
                    topic: "other_topic".to_string(),
                    data: vec![13, 14, 15, 16],
                },
                origin: [3; 32],
            },
        ]
    }

    /// Helper to create archive with pipeline config
    async fn create_archive_with_config(
        config_json: &str,
        dir_prefix: &str,
    ) -> (Archive, NamedTempFile) {
        let config_file = create_config_file(config_json);
        let temp_dir = test_dir(dir_prefix);
        let params = ArchiveParams {
            pipelines_config_path: Some(config_file.path().to_path_buf()),
            ..Default::default()
        };
        let archive =
            Archive::create_or_open_with_conf(&temp_dir, params).await;
        (archive, config_file)
    }

    /// Minimal pipeline config for moonlight-only filtering
    fn moonlight_config(table: &str) -> String {
        format!(
            r#"{{
            "version": 1,
            "pipelines": [{{
                "id": "test",
                "type": "sql_event_table",
                "enabled": true,
                "required": false,
                "filter": {{ "contract_ids": [], "topics": ["moonlight"] }},
                "sink": {{
                    "kind": "sqlite_table",
                    "table": "{}",
                    "schema": [{{ "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true }}],
                    "indexes": []
                }}
            }}]
        }}"#,
            table
        )
    }

    // =========================================================================
    // Core Pipeline Functionality
    // =========================================================================

    #[tokio::test]
    async fn test_finalization_with_filtering_and_checkpoint() {
        // Tests: finalization flow, topic filtering, checkpoint update, no
        // errors, canonical vs pipeline storage
        let config = r#"{
            "version": 1,
            "pipelines": [{
                "id": "moonlight_events",
                "type": "sql_event_table",
                "enabled": true,
                "required": false,
                "filter": { "contract_ids": [], "topics": ["moonlight"] },
                "sink": {
                    "kind": "sqlite_table",
                    "table": "moonlight_events",
                    "schema": [
                        { "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true },
                        { "name": "source", "type": "TEXT", "primary_key": false, "not_null": true },
                        { "name": "data", "type": "BLOB", "primary_key": false, "not_null": true }
                    ],
                    "indexes": [{ "name": "idx_topic", "columns": ["topic"], "unique": false }]
                }
            }]
        }"#;

        let (mut archive, _cfg) =
            create_archive_with_config(config, "finalization").await;

        archive
            .store_unfinalized_events(1, [5; 32], dummy_events())
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await
            .unwrap();

        // Verify: 4 events in canonical, 2 moonlight in pipeline
        let canonical: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM finalized_events WHERE block_height = 1",
        )
        .fetch_one(&archive.sqlite_reader)
        .await
        .unwrap();
        assert_eq!(canonical, 4, "All events in canonical storage");

        let pipeline: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM moonlight_events")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(pipeline, 2, "Only moonlight events in pipeline table");

        // Verify checkpoint and no errors
        let (checkpoint, error): (i64, Option<String>) = sqlx::query_as(
            "SELECT last_processed_height, last_error FROM pipeline_meta WHERE pipeline_id = ?",
        )
        .bind("moonlight_events")
        .fetch_one(&archive.sqlite_reader)
        .await
        .unwrap();
        assert_eq!(checkpoint, 1);
        assert!(error.is_none());
    }

    #[tokio::test]
    async fn test_contract_and_combined_filtering() {
        // Tests: contract-only filter, AND logic (contract + topic)
        let transfer = dusk_core::transfer::TRANSFER_CONTRACT;
        let config = format!(
            r#"{{
            "version": 1,
            "pipelines": [
                {{
                    "id": "transfer_only",
                    "type": "sql_event_table",
                    "enabled": true,
                    "required": false,
                    "filter": {{ "contract_ids": ["{}"], "topics": [] }},
                    "sink": {{
                        "kind": "sqlite_table",
                        "table": "transfer_events",
                        "schema": [{{ "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true }}],
                        "indexes": []
                    }}
                }},
                {{
                    "id": "transfer_phoenix",
                    "type": "sql_event_table",
                    "enabled": true,
                    "required": false,
                    "filter": {{ "contract_ids": ["{}"], "topics": ["phoenix"] }},
                    "sink": {{
                        "kind": "sqlite_table",
                        "table": "phoenix_events",
                        "schema": [{{ "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true }}],
                        "indexes": []
                    }}
                }}
            ]
        }}"#,
            transfer, transfer
        );

        let (mut archive, _cfg) =
            create_archive_with_config(&config, "contract-filter").await;

        archive
            .store_unfinalized_events(1, [5; 32], dummy_events())
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await
            .unwrap();

        // Contract-only: 3 events from transfer contract
        let transfer_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM transfer_events")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(transfer_count, 3, "3 events from transfer contract");

        // AND filter: only phoenix from transfer = 1
        let phoenix_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM phoenix_events")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(phoenix_count, 1, "Only phoenix from transfer contract");
    }

    #[tokio::test]
    async fn test_multiple_pipelines_and_sequential_blocks() {
        // Tests: multiple pipelines, sequential block processing
        let config = r#"{
            "version": 1,
            "pipelines": [
                {
                    "id": "moonlight_pipe",
                    "type": "sql_event_table",
                    "enabled": true,
                    "required": false,
                    "filter": { "contract_ids": [], "topics": ["moonlight"] },
                    "sink": {
                        "kind": "sqlite_table",
                        "table": "moonlight_tbl",
                        "schema": [{ "name": "data", "type": "BLOB", "primary_key": false, "not_null": true }],
                        "indexes": []
                    }
                },
                {
                    "id": "phoenix_pipe",
                    "type": "sql_event_table",
                    "enabled": true,
                    "required": false,
                    "filter": { "contract_ids": [], "topics": ["phoenix"] },
                    "sink": {
                        "kind": "sqlite_table",
                        "table": "phoenix_tbl",
                        "schema": [{ "name": "data", "type": "BLOB", "primary_key": false, "not_null": true }],
                        "indexes": []
                    }
                }
            ]
        }"#;

        let (mut archive, _cfg) =
            create_archive_with_config(config, "multi-seq").await;

        // Process 3 blocks
        for height in 1..=3u64 {
            let hash = [height as u8; 32];
            archive
                .store_unfinalized_events(height, hash, dummy_events())
                .await
                .unwrap();
            archive
                .finalize_archive_data(height, &hex::encode(hash))
                .await
                .unwrap();
        }

        // 2 moonlight * 3 blocks = 6
        let moonlight: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM moonlight_tbl")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(moonlight, 6);

        // 1 phoenix * 3 blocks = 3
        let phoenix: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM phoenix_tbl")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(phoenix, 3);

        // Checkpoint at block 3 for both
        for pid in ["moonlight_pipe", "phoenix_pipe"] {
            let cp: i64 = sqlx::query_scalar(
                "SELECT last_processed_height FROM pipeline_meta WHERE pipeline_id = ?",
            )
            .bind(pid)
            .fetch_one(&archive.sqlite_reader)
            .await
            .unwrap();
            assert_eq!(cp, 3);
        }
    }

    // =========================================================================
    // Edge Cases and Error Handling
    // =========================================================================

    #[tokio::test]
    async fn test_idempotency_double_finalization_rejected() {
        let config = moonlight_config("idem_table");
        let (mut archive, _cfg) =
            create_archive_with_config(&config, "idempotency").await;

        archive
            .store_unfinalized_events(1, [5; 32], dummy_events())
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await
            .unwrap();

        let count_before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM idem_table")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();

        // Second finalization should fail
        let result = archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await;
        assert!(result.is_err(), "Double finalization should error");

        // No duplicates inserted
        let count_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM idem_table")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(count_before, count_after);
    }

    #[tokio::test]
    async fn test_empty_block_and_disabled_pipeline() {
        let config = r#"{
            "version": 1,
            "pipelines": [
                {
                    "id": "enabled",
                    "type": "sql_event_table",
                    "enabled": true,
                    "required": false,
                    "filter": { "contract_ids": [], "topics": ["moonlight"] },
                    "sink": {
                        "kind": "sqlite_table",
                        "table": "enabled_tbl",
                        "schema": [{ "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true }],
                        "indexes": []
                    }
                },
                {
                    "id": "disabled",
                    "type": "sql_event_table",
                    "enabled": false,
                    "required": false,
                    "filter": { "contract_ids": [], "topics": ["moonlight"] },
                    "sink": {
                        "kind": "sqlite_table",
                        "table": "disabled_tbl",
                        "schema": [{ "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true }],
                        "indexes": []
                    }
                }
            ]
        }"#;

        let (mut archive, _cfg) =
            create_archive_with_config(config, "empty-disabled").await;

        // Block 1: empty
        archive
            .store_unfinalized_events(1, [1; 32], vec![])
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([1; 32]))
            .await
            .unwrap();

        // Block 2: with events
        archive
            .store_unfinalized_events(2, [2; 32], dummy_events())
            .await
            .unwrap();
        archive
            .finalize_archive_data(2, &hex::encode([2; 32]))
            .await
            .unwrap();

        // Enabled pipeline: checkpoint at 2, has 2 moonlight events
        let (cp, count): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT last_processed_height FROM pipeline_meta WHERE pipeline_id = 'enabled'),
                    (SELECT COUNT(*) FROM enabled_tbl)",
        )
        .fetch_one(&archive.sqlite_reader)
        .await
        .unwrap();
        assert_eq!(cp, 2);
        assert_eq!(count, 2);

        // Disabled pipeline: table should not exist
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='disabled_tbl')",
        )
        .fetch_one(&archive.sqlite_reader)
        .await
        .unwrap();
        assert!(!exists, "Disabled pipeline should not create table");
    }

    #[tokio::test]
    async fn test_no_pipeline_config() {
        // Without pipeline config, no pipeline_meta should exist
        let temp_dir = test_dir("no-config");
        let params = ArchiveParams {
            pipelines_config_path: None,
            ..Default::default()
        };
        let mut archive =
            Archive::create_or_open_with_conf(&temp_dir, params).await;

        archive
            .store_unfinalized_events(1, [5; 32], dummy_events())
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await
            .unwrap();

        let result: Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM pipeline_meta")
                .fetch_one(&archive.sqlite_reader)
                .await;

        match result {
            Ok(count) => assert_eq!(count, 0),
            Err(_) => {} // Table doesn't exist, also fine
        }
    }

    // =========================================================================
    // Data Integrity
    // =========================================================================

    #[tokio::test]
    async fn test_data_integrity_and_column_values() {
        // Tests: data blob integrity, block_hash, origin, event_ordinal,
        // inserted_at
        let config = r#"{
            "version": 1,
            "pipelines": [{
                "id": "integrity",
                "type": "sql_event_table",
                "enabled": true,
                "required": false,
                "filter": { "contract_ids": [], "topics": ["moonlight"] },
                "sink": {
                    "kind": "sqlite_table",
                    "table": "integrity_tbl",
                    "schema": [
                        { "name": "topic", "type": "TEXT", "primary_key": false, "not_null": true },
                        { "name": "data", "type": "BLOB", "primary_key": false, "not_null": true }
                    ],
                    "indexes": []
                }
            }]
        }"#;

        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let (mut archive, _cfg) =
            create_archive_with_config(config, "integrity").await;

        let block_hash = [0xAB; 32];
        let events = dummy_events();
        let expected_data: Vec<Vec<u8>> = events
            .iter()
            .filter(|e| e.event.topic == "moonlight")
            .map(|e| e.event.data.clone())
            .collect();

        archive
            .store_unfinalized_events(1, block_hash, events)
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode(block_hash))
            .await
            .unwrap();

        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Fetch all columns
        let rows: Vec<(i64, String, String, i64, Vec<u8>, i64)> = sqlx::query_as(
            "SELECT block_height, block_hash, origin, event_ordinal, data, inserted_at
             FROM integrity_tbl ORDER BY event_ordinal",
        )
        .fetch_all(&archive.sqlite_reader)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);

        for (i, row) in rows.iter().enumerate() {
            // block_height
            assert_eq!(row.0, 1);
            // block_hash as hex
            assert_eq!(row.1, hex::encode(block_hash));
            // event_ordinal sequential
            assert_eq!(row.3, i as i64);
            // data integrity
            assert_eq!(row.4, expected_data[i]);
            // inserted_at within bounds
            assert!(row.5 >= before && row.5 <= after);
        }

        // origins stored as hex: [1;32] and [2;32]
        assert_eq!(rows[0].2, hex::encode([1u8; 32]));
        assert_eq!(rows[1].2, hex::encode([2u8; 32]));
    }

    #[tokio::test]
    async fn test_large_batch_processing() {
        let transfer = dusk_core::transfer::TRANSFER_CONTRACT;
        let large_events: Vec<ContractTxEvent> = (0..100)
            .map(|i| ContractTxEvent {
                event: ContractEvent {
                    target: transfer,
                    topic: "moonlight".to_string(),
                    data: vec![i as u8; 64],
                },
                origin: [i as u8; 32],
            })
            .collect();

        let config = moonlight_config("large_batch_tbl");
        let (mut archive, _cfg) =
            create_archive_with_config(&config, "large-batch").await;

        archive
            .store_unfinalized_events(1, [5; 32], large_events)
            .await
            .unwrap();
        archive
            .finalize_archive_data(1, &hex::encode([5; 32]))
            .await
            .unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM large_batch_tbl")
                .fetch_one(&archive.sqlite_reader)
                .await
                .unwrap();
        assert_eq!(count, 100);
    }
}
