// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Generic SQL event table pipeline.
//!
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_trait::async_trait;
use dusk_core::abi::ContractId;
use sqlx::SqlitePool;

use tracing::{debug, warn};

use crate::archive::pipeline::{Pipeline, PipelineContext, PipelineRunStats};
use crate::archive::conf::pipeline_config::{ColumnDef, PipelineConfig};
use crate::archive::schema_manager::SchemaManager;

/// Generic pipeline that filters events and inserts them into a SQL table.
#[derive(Debug)]
pub struct SqlEventTablePipeline {
    config: PipelineConfig,
}

impl SqlEventTablePipeline {
    pub fn new(config: PipelineConfig) -> Result<Self> {
        // Validate config has required fields
        if config.sink.is_none() {
            return Err(anyhow::anyhow!(
                "SqlEventTablePipeline requires sink configuration"
            ));
        }
        if config.filter.is_none() {
            return Err(anyhow::anyhow!(
                "SqlEventTablePipeline requires filter configuration"
            ));
        }

        Ok(Self { config })
    }

    /// Check if an event matches the filter criteria.
    fn matches_filter(&self, contract_id: &ContractId, topic: &str) -> bool {
        let filter = self.config.filter.as_ref().unwrap();

        // Check contract ID filter
        let contract_id_str = contract_id.to_string();
        let contract_match = filter.contract_ids.is_empty()
            || filter.contract_ids.iter().any(|id| id == &contract_id_str);

        // Check topic filter
        let topic_match = filter.topics.is_empty()
            || filter.topics.iter().any(|t| t == topic);

        contract_match && topic_match
    }
}

#[async_trait]
impl Pipeline for SqlEventTablePipeline {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn pipeline_type(&self) -> &'static str {
        "sql_event_table"
    }

    async fn ensure_schema(&self, writer: &SqlitePool) -> Result<()> {
        let sink = self.config.sink.as_ref().unwrap();

        // Reserved columns + user-defined columns
        let col = |name: &str, col_type: &str, pk: bool| ColumnDef {
            name: name.into(),
            col_type: col_type.into(),
            primary_key: pk,
            not_null: true,
        };

        let mut schema = vec![
            col("block_height", "INTEGER", true),
            col("block_hash", "TEXT", false),
            col("origin", "TEXT", true),
            col("event_ordinal", "INTEGER", true),
            col("inserted_at", "INTEGER", false),
        ];
        schema.extend(sink.schema.clone());

        SchemaManager::ensure_table(writer, &sink.table, &schema, &sink.indexes)
            .await
    }

    async fn run_for_block(
        &self,
        writer: &SqlitePool,
        ctx: &PipelineContext<'_>,
    ) -> Result<PipelineRunStats> {
        let sink = self.config.sink.as_ref().unwrap();
        let decoder = self.config.decoder.as_ref();

        let mut stats = PipelineRunStats::default();
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        // Iterate through all events in the block
        let mut event_ordinal = 0i64;

        for (ident, events) in ctx.grouped_events {
            let origin_hex = hex::encode(ident.origin());

            for event in events {
                // Check if event matches filter
                if !self.matches_filter(&event.target, &event.topic) {
                    continue;
                }

                stats.events_processed += 1;

                // Decode event data if decoder configured
                let decoded_json: Option<serde_json::Value> = if let Some(dec) =
                    decoder
                {
                    match dec.mode.as_str() {
                        "raw_only" => None,
                        "raw_plus_json" | "decoded_only" => {
                            // TODO: Integrate data-driver decoding here
                            // For now, store NULL for decoded_json or
                            // decoded_only
                            warn!("Data-driver decoding not yet implemented, storing NULL");
                            stats.decode_failures += 1;
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                // Check which optional columns are in user schema
                let has = |name| sink.schema.iter().any(|c| c.name == name);
                let (has_topic, has_source, has_data) =
                    (has("topic"), has("source"), has("data"));

                // Build INSERT statement dynamically
                // Note: We use runtime SQL here because table/column names are
                // dynamic
                let mut col_names = vec![
                    "block_height",
                    "block_hash",
                    "origin",
                    "event_ordinal",
                    "inserted_at",
                ];
                let mut placeholders = vec!["?", "?", "?", "?", "?"];

                if has_topic {
                    col_names.push("topic");
                    placeholders.push("?");
                }
                if has_source {
                    col_names.push("source");
                    placeholders.push("?");
                }
                if has_data {
                    col_names.push("data");
                    placeholders.push("?");
                }

                if decoded_json.is_some() {
                    col_names.push("payload_json");
                    placeholders.push("?");
                }

                let insert_sql = format!(
                    "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT DO NOTHING",
                    sink.table,
                    col_names.join(", "),
                    placeholders.join(", ")
                );

                let source_hex = event.target.to_string();
                let block_height_i64 = ctx.block_height as i64;

                let mut query = sqlx::query(&insert_sql)
                    .bind(block_height_i64)
                    .bind(ctx.block_hash)
                    .bind(&origin_hex)
                    .bind(event_ordinal)
                    .bind(now);

                if has_topic {
                    query = query.bind(&event.topic);
                }
                if has_source {
                    query = query.bind(&source_hex);
                }
                if has_data {
                    query = query.bind(&event.data);
                }

                if let Some(ref json) = decoded_json {
                    let json_str = serde_json::to_string(json)?;
                    query = query.bind(json_str);
                }

                let result =
                    query.execute(writer).await.with_context(|| {
                        format!(
                            "Failed to insert event into table '{}'",
                            sink.table
                        )
                    })?;

                stats.rows_affected += result.rows_affected();
                event_ordinal += 1;
            }
        }

        debug!(
            "Pipeline '{}': processed {} events, inserted {} rows",
            self.id(),
            stats.events_processed,
            stats.rows_affected
        );

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusk_core::abi::ContractId;

    fn create_pipeline(
        contract_ids: Vec<String>,
        topics: Vec<String>,
    ) -> SqlEventTablePipeline {
        use super::super::pipeline_config::{FilterConfig, SinkConfig};

        SqlEventTablePipeline::new(PipelineConfig {
            id: "test".to_string(),
            pipeline_type: "sql_event_table".to_string(),
            enabled: true,
            required: false,
            source: None,
            filter: Some(FilterConfig {
                contract_ids,
                topics,
            }),
            decoder: None,
            sink: Some(SinkConfig {
                kind: "sqlite_table".to_string(),
                table: "test_table".to_string(),
                schema: vec![],
                indexes: vec![],
            }),
            params: serde_json::Value::Null,
        })
        .unwrap()
    }

    // =========================================================================
    // Filter Logic
    // =========================================================================

    #[test]
    fn test_filter_empty_matches_all() {
        let p = create_pipeline(vec![], vec![]);
        let c1 = ContractId::from_bytes([1; 32]);
        let c2 = ContractId::from_bytes([2; 32]);

        assert!(p.matches_filter(&c1, "any"));
        assert!(p.matches_filter(&c2, "other"));
    }

    #[test]
    fn test_filter_contract_only() {
        let c1 = ContractId::from_bytes([1; 32]);
        let c2 = ContractId::from_bytes([2; 32]);
        let p = create_pipeline(vec![c1.to_string()], vec![]);

        assert!(p.matches_filter(&c1, "any_topic"));
        assert!(!p.matches_filter(&c2, "any_topic"));
    }

    #[test]
    fn test_filter_topic_only() {
        let p = create_pipeline(vec![], vec!["moonlight".to_string()]);
        let c = ContractId::from_bytes([1; 32]);

        assert!(p.matches_filter(&c, "moonlight"));
        assert!(!p.matches_filter(&c, "phoenix"));
    }

    #[test]
    fn test_filter_and_or_logic() {
        // (C1 OR C2) AND (moonlight OR phoenix)
        let c1 = ContractId::from_bytes([1; 32]);
        let c2 = ContractId::from_bytes([2; 32]);
        let c3 = ContractId::from_bytes([3; 32]);

        let p = create_pipeline(
            vec![c1.to_string(), c2.to_string()],
            vec!["moonlight".to_string(), "phoenix".to_string()],
        );

        // Valid: matching contract AND matching topic
        assert!(p.matches_filter(&c1, "moonlight"));
        assert!(p.matches_filter(&c2, "phoenix"));

        // Invalid: wrong topic
        assert!(!p.matches_filter(&c1, "convert"));

        // Invalid: wrong contract
        assert!(!p.matches_filter(&c3, "moonlight"));
    }

    #[test]
    fn test_filter_case_sensitive_topics() {
        let p = create_pipeline(vec![], vec!["Moonlight".to_string()]);
        let c = ContractId::from_bytes([1; 32]);

        assert!(p.matches_filter(&c, "Moonlight"));
        assert!(!p.matches_filter(&c, "moonlight"));
        assert!(!p.matches_filter(&c, "MOONLIGHT"));
    }

    #[test]
    fn test_filter_special_chars_and_whitespace() {
        let p = create_pipeline(
            vec![],
            vec![
                "event:v1".to_string(),
                "topic with spaces".to_string(),
                "".to_string(),
            ],
        );
        let c = ContractId::from_bytes([1; 32]);

        assert!(p.matches_filter(&c, "event:v1"));
        assert!(p.matches_filter(&c, "topic with spaces"));
        assert!(p.matches_filter(&c, "")); // empty topic
        assert!(!p.matches_filter(&c, "event_v1"));
    }

    // =========================================================================
    // Constructor Validation
    // =========================================================================

    #[test]
    fn test_requires_filter_and_sink() {
        use super::super::pipeline_config::{FilterConfig, SinkConfig};

        // Missing sink
        let no_sink = SqlEventTablePipeline::new(PipelineConfig {
            id: "test".to_string(),
            pipeline_type: "sql_event_table".to_string(),
            enabled: true,
            required: false,
            source: None,
            filter: Some(FilterConfig {
                contract_ids: vec![],
                topics: vec![],
            }),
            decoder: None,
            sink: None,
            params: serde_json::Value::Null,
        });
        assert!(no_sink.is_err());
        assert!(no_sink.unwrap_err().to_string().contains("sink"));

        // Missing filter
        let no_filter = SqlEventTablePipeline::new(PipelineConfig {
            id: "test".to_string(),
            pipeline_type: "sql_event_table".to_string(),
            enabled: true,
            required: false,
            source: None,
            filter: None,
            decoder: None,
            sink: Some(SinkConfig {
                kind: "sqlite_table".to_string(),
                table: "test".to_string(),
                schema: vec![],
                indexes: vec![],
            }),
            params: serde_json::Value::Null,
        });
        assert!(no_filter.is_err());
        assert!(no_filter.unwrap_err().to_string().contains("filter"));
    }

    #[test]
    fn test_pipeline_id_and_type() {
        let p = create_pipeline(vec![], vec!["test".to_string()]);
        assert_eq!(p.id(), "test");
        assert_eq!(p.pipeline_type(), "sql_event_table");
    }
}
