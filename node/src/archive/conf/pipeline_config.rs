// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Pipeline configuration loading and validation.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

/// Regex for valid SQL identifiers (table/column/index names).
const IDENTIFIER_REGEX: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

/// Top-level pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinesConfig {
    /// Config version (must be 1 for now).
    pub version: u32,

    /// List of pipeline configurations.
    pub pipelines: Vec<PipelineConfig>,
}

/// Configuration for a single pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Unique pipeline identifier (must match identifier regex).
    pub id: String,

    /// Pipeline type: "moonlight_builtin" or "sql_event_table".
    #[serde(rename = "type")]
    pub pipeline_type: String,

    /// Whether this pipeline is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether pipeline failures should halt finalization.
    #[serde(default)]
    pub required: bool,

    /// Source configuration (optional, used by generic pipelines).
    #[serde(default)]
    pub source: Option<SourceConfig>,

    /// Filter configuration (optional, used by generic pipelines).
    #[serde(default)]
    pub filter: Option<FilterConfig>,

    /// Decoder configuration (optional, used by generic pipelines).
    #[serde(default)]
    pub decoder: Option<DecoderConfig>,

    /// Sink configuration (optional, used by generic pipelines).
    #[serde(default)]
    pub sink: Option<SinkConfig>,

    /// Pipeline-specific parameters (optional).
    #[serde(default)]
    pub params: serde_json::Value,
}

fn default_true() -> bool {
    true
}

/// Source stream configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Source stream: "finalized_events" (default).
    ///
    /// Other options may be added in the future. For now you can only use
    /// "finalized_events" as the source of events to process in the pipeline.
    #[serde(default = "default_stream")]
    pub stream: String,

    /// Event grouping mode for pipeline processing.
    ///
    /// - `"per_event"` (default): Process each event individually. The
    ///   pipeline receives events one at a time. Use this when events are
    ///   independent and can be processed in isolation (e.g., logging
    ///   individual transfers).
    ///
    /// - `"per_origin"`: Group events by their origin (transaction hash)
    ///   before processing. The pipeline receives all events from the same
    ///   transaction together. Use this when you need transaction-level
    ///   context or want to process related events atomically (e.g.,
    ///   aggregating all state changes from a single transaction).
    #[serde(default = "default_grouping")]
    pub grouping: String,
}

fn default_stream() -> String {
    "finalized_events".to_string()
}

fn default_grouping() -> String {
    "per_event".to_string()
}

/// Event filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Contract IDs to filter (hex strings).
    #[serde(default)]
    pub contract_ids: Vec<String>,

    /// Topics to filter.
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Decoder configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderConfig {
    /// Decode mode: "raw_only", "raw_plus_json", "decoded_only".
    ///
    /// - "raw_only": Store only the raw event data as a BLOB.
    /// - "raw_plus_json": Store both raw data and a JSON representation.
    /// - "decoded_only": Store only the decoded event fields in separate
    ///   columns for each event field in the decoded event "struct".
    ///     TODO: raw_plus_json and decoded_only are not yet implemented. Need
    ///       data-driver decoding here.
    ///     TODO: find out if table columns for
    ///       decoded_only are auto-generated or already defined in sink schema &
    ///       expected to match  the decoded event fields.
    #[serde(default = "default_decode_mode")]
    pub mode: String,

    /// Data-driver ID (optional, e.g., "transfer").
    #[serde(default)]
    pub data_driver: Option<String>,

    /// Event type to decode (optional, used with data-driver).
    #[serde(default)]
    pub event_type: Option<String>,
}

fn default_decode_mode() -> String {
    "raw_only".to_string()
}

/// Sink configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    /// Sink kind: "sqlite_table".
    pub kind: String,

    /// Table name (must match identifier regex).
    pub table: String,

    /// Table schema (list of columns).
    pub schema: Vec<ColumnDef>,

    /// Indexes to create (optional).
    #[serde(default)]
    pub indexes: Vec<IndexDef>,
}

/// Column definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    /// Column name (must match identifier regex).
    pub name: String,

    /// SQL column type: INTEGER, TEXT, BLOB, REAL.
    #[serde(rename = "type")]
    pub col_type: String,

    /// Whether column is part of the primary key.
    #[serde(default)]
    pub primary_key: bool,

    /// Whether column is NOT NULL.
    #[serde(default)]
    pub not_null: bool,
}

/// Index definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDef {
    /// Index name (must match identifier regex).
    pub name: String,

    /// Columns to index.
    pub columns: Vec<String>,

    /// Whether index is unique.
    #[serde(default)]
    pub unique: bool,
}

impl PipelinesConfig {
    /// Load configuration from a JSON file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).with_context(|| {
            format!("Failed to read config file: {:?}", path)
        })?;

        let config: Self =
            serde_json::from_str(&content).with_context(|| {
                format!("Failed to parse config file: {:?}", path)
            })?;

        config.validate()?;

        Ok(config)
    }

    /// Validate the entire configuration.
    pub fn validate(&self) -> Result<()> {
        // Check version
        if self.version != 1 {
            return Err(anyhow!(
                "Unsupported config version: {}. Only version 1 is supported.",
                self.version
            ));
        }

        // Check for duplicate pipeline IDs
        let mut ids = HashSet::new();
        for pipeline in &self.pipelines {
            if !ids.insert(&pipeline.id) {
                return Err(anyhow!("Duplicate pipeline ID: {}", pipeline.id));
            }
        }

        // Validate each pipeline
        for pipeline in &self.pipelines {
            pipeline.validate()?;
        }

        Ok(())
    }
}

impl PipelineConfig {
    /// Validate a single pipeline configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate pipeline ID
        validate_identifier(&self.id, "pipeline ID")?;

        // Validate pipeline type
        match self.pipeline_type.as_str() {
            "moonlight_builtin" => {
                // No additional validation needed for built-in type
            }
            "sql_event_table" => {
                // Validate required fields for generic pipeline
                let filter = self.filter.as_ref()
                    .ok_or_else(|| anyhow!("Pipeline '{}': 'filter' is required for sql_event_table pipelines", self.id))?;

                let sink = self.sink.as_ref()
                    .ok_or_else(|| anyhow!("Pipeline '{}': 'sink' is required for sql_event_table pipelines", self.id))?;

                // Validate filter
                if filter.contract_ids.is_empty() && filter.topics.is_empty() {
                    return Err(anyhow!("Pipeline '{}': filter must specify at least one contract_id or topic", self.id));
                }

                // Validate sink
                sink.validate(&self.id)?;
            }
            other => {
                return Err(anyhow!(
                    "Pipeline '{}': unsupported pipeline type '{}'",
                    self.id,
                    other
                ));
            }
        }

        Ok(())
    }
}

impl SinkConfig {
    /// Validate sink configuration.
    pub fn validate(&self, pipeline_id: &str) -> Result<()> {
        // Validate kind
        if self.kind != "sqlite_table" {
            return Err(anyhow!(
                "Pipeline '{}': unsupported sink kind '{}'",
                pipeline_id,
                self.kind
            ));
        }

        // Validate table name
        validate_identifier(
            &self.table,
            &format!("Pipeline '{}' table name", pipeline_id),
        )?;

        // Reserved table names
        const RESERVED_TABLES: &[&str] = &[
            "blocks",
            "finalized_events",
            "unfinalized_events",
            "transactions",
            "pipeline_meta",
            "sqlite_master",
            "sqlite_sequence",
        ];
        let table_lower = self.table.to_lowercase();
        if RESERVED_TABLES.iter().any(|r| *r == table_lower) {
            return Err(anyhow!(
                "Pipeline '{}': table name '{}' is reserved for internal use",
                pipeline_id,
                self.table
            ));
        }

        // Validate columns
        if self.schema.is_empty() {
            return Err(anyhow!(
                "Pipeline '{}': sink schema must have at least one column",
                pipeline_id
            ));
        }

        // Reserved column names (auto-added by the pipeline)
        const RESERVED_COLUMNS: &[&str] =
            &["block_height", "origin", "event_ordinal", "inserted_at"];

        let mut col_names = HashSet::new();
        for col in &self.schema {
            validate_identifier(
                &col.name,
                &format!("Pipeline '{}' column name", pipeline_id),
            )?;

            let col_lower = col.name.to_lowercase();
            if RESERVED_COLUMNS.iter().any(|r| *r == col_lower) {
                return Err(anyhow!(
                    "Pipeline '{}': column name '{}' is reserved (auto-added by system)",
                    pipeline_id,
                    col.name
                ));
            }

            if !col_names.insert(&col.name) {
                return Err(anyhow!(
                    "Pipeline '{}': duplicate column name '{}'",
                    pipeline_id,
                    col.name
                ));
            }

            // Validate column type
            match col.col_type.to_uppercase().as_str() {
                "INTEGER" | "TEXT" | "BLOB" | "REAL" | "JSON" => {}
                other => {
                    return Err(anyhow!("Pipeline '{}': unsupported column type '{}' for column '{}'", pipeline_id, other, col.name));
                }
            }
        }

        // Validate indexes
        for idx in &self.indexes {
            validate_identifier(
                &idx.name,
                &format!("Pipeline '{}' index name", pipeline_id),
            )?;

            if idx.columns.is_empty() {
                return Err(anyhow!(
                    "Pipeline '{}': index '{}' must have at least one column",
                    pipeline_id,
                    idx.name
                ));
            }

            for col in &idx.columns {
                if !col_names.contains(col) {
                    return Err(anyhow!("Pipeline '{}': index '{}' references non-existent column '{}'", pipeline_id, idx.name, col));
                }
            }
        }

        Ok(())
    }
}

/// Validate an identifier matches the required pattern.
fn validate_identifier(id: &str, context: &str) -> Result<()> {
    let re = regex::Regex::new(IDENTIFIER_REGEX).unwrap();
    if !re.is_match(id) {
        return Err(anyhow!(
            "{} '{}' must match pattern {} (alphanumeric and underscore, starting with letter or underscore)",
            context, id, IDENTIFIER_REGEX
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a minimal valid pipeline config
    fn minimal_pipeline(id: &str, table: &str) -> PipelineConfig {
        PipelineConfig {
            id: id.to_string(),
            pipeline_type: "sql_event_table".to_string(),
            enabled: true,
            required: false,
            source: None,
            filter: Some(FilterConfig {
                contract_ids: vec![],
                topics: vec!["test".to_string()],
            }),
            decoder: None,
            sink: Some(SinkConfig {
                kind: "sqlite_table".to_string(),
                table: table.to_string(),
                schema: vec![ColumnDef {
                    name: "col".to_string(),
                    col_type: "TEXT".to_string(),
                    primary_key: false,
                    not_null: false,
                }],
                indexes: vec![],
            }),
            params: serde_json::Value::Null,
        }
    }

    /// Helper to create config with custom sink
    fn pipeline_with_sink(sink: SinkConfig) -> PipelinesConfig {
        PipelinesConfig {
            version: 1,
            pipelines: vec![PipelineConfig {
                id: "test".to_string(),
                pipeline_type: "sql_event_table".to_string(),
                enabled: true,
                required: false,
                source: None,
                filter: Some(FilterConfig {
                    contract_ids: vec![],
                    topics: vec!["test".to_string()],
                }),
                decoder: None,
                sink: Some(sink),
                params: serde_json::Value::Null,
            }],
        }
    }

    // =========================================================================
    // Identifier Validation
    // =========================================================================

    #[test]
    fn test_identifier_validation() {
        // Valid identifiers
        for valid in ["valid_name", "_valid", "valid123", "A", "_", "a1b2c3"] {
            assert!(validate_identifier(valid, "test").is_ok(), "{}", valid);
        }

        // Invalid: starts with number, contains special chars
        let invalid = [
            "123invalid",
            "invalid-name",
            "invalid name",
            "test;drop",
            "test'quote",
            "test\"quote",
            "test`tick",
            "test.dot",
            "test\ttab",
            "test\nnewline",
            "",
            "tëst",
            "测试",
        ];
        for inv in invalid {
            assert!(validate_identifier(inv, "test").is_err(), "{}", inv);
        }

        // SQL keywords are valid (handled by quoting)
        for keyword in ["select", "table", "index", "from", "where"] {
            assert!(validate_identifier(keyword, "test").is_ok());
        }
    }

    // =========================================================================
    // Config Loading
    // =========================================================================

    #[test]
    fn test_config_load_valid_and_invalid() {
        // Valid config
        let valid_json = r#"{
            "version": 1,
            "pipelines": [{
                "id": "test_pipeline",
                "type": "sql_event_table",
                "enabled": true,
                "filter": { "contract_ids": [], "topics": ["moonlight"] },
                "sink": {
                    "kind": "sqlite_table",
                    "table": "test_table",
                    "schema": [{ "name": "topic", "type": "TEXT" }],
                    "indexes": []
                }
            }]
        }"#;

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(valid_json.as_bytes()).unwrap();
        let config = PipelinesConfig::load(temp.path()).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.pipelines[0].id, "test_pipeline");

        // Invalid JSON
        let mut temp2 = NamedTempFile::new().unwrap();
        temp2.write_all(b"{ invalid json }").unwrap();
        assert!(PipelinesConfig::load(temp2.path()).is_err());
    }

    // =========================================================================
    // Pipeline Validation
    // =========================================================================

    #[test]
    fn test_version_and_duplicate_ids() {
        // Wrong version
        let bad_version = PipelinesConfig {
            version: 99,
            pipelines: vec![],
        };
        assert!(bad_version
            .validate()
            .unwrap_err()
            .to_string()
            .contains("version"));

        // Duplicate IDs
        let duplicates = PipelinesConfig {
            version: 1,
            pipelines: vec![
                minimal_pipeline("dupe", "table1"),
                minimal_pipeline("dupe", "table2"),
            ],
        };
        assert!(duplicates
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Duplicate"));

        // Empty pipelines is valid
        let empty = PipelinesConfig {
            version: 1,
            pipelines: vec![],
        };
        assert!(empty.validate().is_ok());
    }

    #[test]
    fn test_pipeline_type_validation() {
        // Unknown type
        let mut p = minimal_pipeline("test", "tbl");
        p.pipeline_type = "unknown_type".to_string();
        let config = PipelinesConfig {
            version: 1,
            pipelines: vec![p],
        };
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("pipeline type"));

        // moonlight_builtin needs no filter/sink
        let builtin = PipelinesConfig {
            version: 1,
            pipelines: vec![PipelineConfig {
                id: "moonlight".to_string(),
                pipeline_type: "moonlight_builtin".to_string(),
                enabled: true,
                required: false,
                source: None,
                filter: None,
                decoder: None,
                sink: None,
                params: serde_json::Value::Null,
            }],
        };
        assert!(builtin.validate().is_ok());
    }

    #[test]
    fn test_invalid_identifiers_rejected() {
        // Invalid pipeline ID
        let mut p = minimal_pipeline("invalid-id", "tbl");
        let config = PipelinesConfig {
            version: 1,
            pipelines: vec![p.clone()],
        };
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("must match pattern"));

        // Invalid table name
        p.id = "valid".to_string();
        p.sink.as_mut().unwrap().table = "invalid-table".to_string();
        let config = PipelinesConfig {
            version: 1,
            pipelines: vec![p.clone()],
        };
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("must match pattern"));

        // Invalid column name
        p.sink.as_mut().unwrap().table = "valid_table".to_string();
        p.sink.as_mut().unwrap().schema[0].name = "invalid col".to_string();
        let config = PipelinesConfig {
            version: 1,
            pipelines: vec![p],
        };
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("must match pattern"));
    }

    // =========================================================================
    // Sink Validation
    // =========================================================================

    #[test]
    fn test_sink_validation() {
        // Unsupported sink kind
        let bad_kind = pipeline_with_sink(SinkConfig {
            kind: "postgres".to_string(),
            table: "tbl".to_string(),
            schema: vec![ColumnDef {
                name: "c".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            }],
            indexes: vec![],
        });
        assert!(bad_kind
            .validate()
            .unwrap_err()
            .to_string()
            .contains("sink kind"));

        // Empty schema
        let empty_schema = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![],
            indexes: vec![],
        });
        assert!(empty_schema
            .validate()
            .unwrap_err()
            .to_string()
            .contains("at least one column"));

        // Duplicate column names
        let dup_cols = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![
                ColumnDef {
                    name: "dup".to_string(),
                    col_type: "TEXT".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "dup".to_string(),
                    col_type: "INTEGER".to_string(),
                    primary_key: false,
                    not_null: false,
                },
            ],
            indexes: vec![],
        });
        assert!(dup_cols
            .validate()
            .unwrap_err()
            .to_string()
            .contains("duplicate column"));

        // Invalid column type
        let bad_type = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![ColumnDef {
                name: "c".to_string(),
                col_type: "INVALID".to_string(),
                primary_key: false,
                not_null: false,
            }],
            indexes: vec![],
        });
        assert!(bad_type
            .validate()
            .unwrap_err()
            .to_string()
            .contains("unsupported column type"));
    }

    #[test]
    fn test_column_types_case_insensitive() {
        // All valid types in various cases
        let config = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![
                ColumnDef {
                    name: "a".to_string(),
                    col_type: "integer".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "b".to_string(),
                    col_type: "Text".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "c".to_string(),
                    col_type: "BLOB".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "d".to_string(),
                    col_type: "real".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "e".to_string(),
                    col_type: "JSON".to_string(),
                    primary_key: false,
                    not_null: false,
                },
            ],
            indexes: vec![],
        });
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_index_validation() {
        // Empty index columns
        let empty_idx = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![ColumnDef {
                name: "col".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            }],
            indexes: vec![IndexDef {
                name: "idx".to_string(),
                columns: vec![],
                unique: false,
            }],
        });
        assert!(empty_idx
            .validate()
            .unwrap_err()
            .to_string()
            .contains("at least one column"));

        // Index references non-existent column
        let bad_ref = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![ColumnDef {
                name: "col".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            }],
            indexes: vec![IndexDef {
                name: "idx".to_string(),
                columns: vec!["nonexistent".to_string()],
                unique: false,
            }],
        });
        assert!(bad_ref
            .validate()
            .unwrap_err()
            .to_string()
            .contains("non-existent column"));
    }

    // =========================================================================
    // Reserved Names
    // =========================================================================

    #[test]
    fn test_reserved_table_names() {
        for reserved in [
            "blocks",
            "finalized_events",
            "pipeline_meta",
            "Blocks",
            "PIPELINE_META",
        ] {
            let config = pipeline_with_sink(SinkConfig {
                kind: "sqlite_table".to_string(),
                table: reserved.to_string(),
                schema: vec![ColumnDef {
                    name: "col".to_string(),
                    col_type: "TEXT".to_string(),
                    primary_key: false,
                    not_null: false,
                }],
                indexes: vec![],
            });
            assert!(
                config
                    .validate()
                    .unwrap_err()
                    .to_string()
                    .contains("reserved"),
                "Should reject: {}",
                reserved
            );
        }

        // Similar but not reserved
        let ok = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "my_blocks".to_string(),
            schema: vec![ColumnDef {
                name: "col".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            }],
            indexes: vec![],
        });
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn test_reserved_column_names() {
        for reserved in [
            "block_height",
            "origin",
            "event_ordinal",
            "inserted_at",
            "ORIGIN",
            "Block_Height",
        ] {
            let config = pipeline_with_sink(SinkConfig {
                kind: "sqlite_table".to_string(),
                table: "tbl".to_string(),
                schema: vec![ColumnDef {
                    name: reserved.to_string(),
                    col_type: "TEXT".to_string(),
                    primary_key: false,
                    not_null: false,
                }],
                indexes: vec![],
            });
            assert!(
                config
                    .validate()
                    .unwrap_err()
                    .to_string()
                    .contains("reserved"),
                "Should reject: {}",
                reserved
            );
        }

        // Similar but not reserved
        let ok = pipeline_with_sink(SinkConfig {
            kind: "sqlite_table".to_string(),
            table: "tbl".to_string(),
            schema: vec![
                ColumnDef {
                    name: "block_height_v2".to_string(),
                    col_type: "INTEGER".to_string(),
                    primary_key: false,
                    not_null: false,
                },
                ColumnDef {
                    name: "origin_hash".to_string(),
                    col_type: "BLOB".to_string(),
                    primary_key: false,
                    not_null: false,
                },
            ],
            indexes: vec![],
        });
        assert!(ok.validate().is_ok());
    }
}
