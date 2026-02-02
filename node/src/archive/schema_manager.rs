// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Schema management for pipeline SQL tables.

use anyhow::{anyhow, Context, Result};
use sqlx::SqlitePool;
use tracing::{debug, info};

use crate::archive::conf::pipeline_config::{ColumnDef, IndexDef};

/// Manages SQL schema creation for pipelines.
pub struct SchemaManager;

impl SchemaManager {
    /// Ensure a table exists for the given sink configuration.
    pub async fn ensure_table(
        pool: &SqlitePool,
        table_name: &str,
        schema: &[ColumnDef],
        indexes: &[IndexDef],
    ) -> Result<()> {
        // Generate CREATE TABLE statement
        let create_table_sql = Self::generate_create_table(table_name, schema)?;

        debug!("Creating table if not exists: {}", table_name);
        debug!("SQL: {}", create_table_sql);

        // Execute CREATE TABLE
        sqlx::query(&create_table_sql)
            .execute(pool)
            .await
            .with_context(|| {
                format!("Failed to create table '{}'", table_name)
            })?;

        info!("Ensured table '{}' exists", table_name);

        // Create indexes
        for index in indexes {
            let create_index_sql =
                Self::generate_create_index(table_name, index)?;

            debug!("Creating index if not exists: {}", index.name);
            debug!("SQL: {}", create_index_sql);

            sqlx::query(&create_index_sql)
                .execute(pool)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create index '{}' on table '{}'",
                        index.name, table_name
                    )
                })?;

            info!(
                "Ensured index '{}' exists on table '{}'",
                index.name, table_name
            );
        }

        Ok(())
    }

    /// Generate CREATE TABLE statement.
    fn generate_create_table(
        table_name: &str,
        schema: &[ColumnDef],
    ) -> Result<String> {
        if schema.is_empty() {
            return Err(anyhow!("Schema must have at least one column"));
        }

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);

        let mut column_defs = Vec::new();
        let mut primary_key_cols = Vec::new();

        for col in schema {
            let mut col_def = format!(
                "    {} {}",
                col.name,
                Self::normalize_col_type(&col.col_type)
            );

            if col.not_null {
                col_def.push_str(" NOT NULL");
            }

            if col.primary_key {
                primary_key_cols.push(col.name.clone());
            }

            column_defs.push(col_def);
        }

        sql.push_str(&column_defs.join(",\n"));

        // Add PRIMARY KEY constraint if any columns are marked as PK
        if !primary_key_cols.is_empty() {
            sql.push_str(",\n    PRIMARY KEY (");
            sql.push_str(&primary_key_cols.join(", "));
            sql.push(')');
        }

        sql.push_str("\n) STRICT");

        Ok(sql)
    }

    /// Generate CREATE INDEX statement.
    fn generate_create_index(
        table_name: &str,
        index: &IndexDef,
    ) -> Result<String> {
        if index.columns.is_empty() {
            return Err(anyhow!("Index must have at least one column"));
        }

        let unique = if index.unique { "UNIQUE " } else { "" };

        let sql = format!(
            "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
            unique,
            index.name,
            table_name,
            index.columns.join(", ")
        );

        Ok(sql)
    }

    /// Normalize column type (handle JSON alias).
    fn normalize_col_type(col_type: &str) -> &str {
        match col_type.to_uppercase().as_str() {
            "JSON" => "TEXT", // Store JSON as TEXT in SQLite
            _ => col_type,
        }
    }
}

/// Ensure the pipeline_meta table exists.
pub async fn ensure_pipeline_meta_table(pool: &SqlitePool) -> Result<()> {
    let sql = r#"
        CREATE TABLE IF NOT EXISTS pipeline_meta (
            pipeline_id TEXT PRIMARY KEY NOT NULL,
            pipeline_type TEXT NOT NULL,
            config_hash TEXT NOT NULL,
            schema_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_processed_height INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            last_error_at INTEGER
        ) STRICT
    "#;

    sqlx::query(sql)
        .execute(pool)
        .await
        .context("Failed to create pipeline_meta table")?;

    info!("Ensured pipeline_meta table exists");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_pool() -> sqlx::SqlitePool {
        // Use in-memory database for tests to avoid file permission issues
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        pool
    }

    #[test]
    fn test_generate_create_table() {
        let schema = vec![
            ColumnDef {
                name: "id".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: true,
                not_null: true,
            },
            ColumnDef {
                name: "data".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
        ];

        let sql = SchemaManager::generate_create_table("test_table", &schema)
            .unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_table"));
        assert!(sql.contains("id INTEGER NOT NULL"));
        assert!(sql.contains("data TEXT"));
        assert!(sql.contains("PRIMARY KEY (id)"));
        assert!(sql.contains("STRICT"));
    }

    #[test]
    fn test_generate_create_index() {
        let index = IndexDef {
            name: "test_idx".to_string(),
            columns: vec!["col1".to_string(), "col2".to_string()],
            unique: false,
        };

        let sql =
            SchemaManager::generate_create_index("test_table", &index).unwrap();
        assert_eq!(
            sql,
            "CREATE INDEX IF NOT EXISTS test_idx ON test_table (col1, col2)"
        );

        let unique_index = IndexDef {
            name: "test_unique_idx".to_string(),
            columns: vec!["col1".to_string()],
            unique: true,
        };

        let sql =
            SchemaManager::generate_create_index("test_table", &unique_index)
                .unwrap();
        assert_eq!(sql, "CREATE UNIQUE INDEX IF NOT EXISTS test_unique_idx ON test_table (col1)");
    }

    #[test]
    fn test_normalize_col_type() {
        assert_eq!(SchemaManager::normalize_col_type("JSON"), "TEXT");
        assert_eq!(SchemaManager::normalize_col_type("INTEGER"), "INTEGER");
        assert_eq!(SchemaManager::normalize_col_type("TEXT"), "TEXT");
        assert_eq!(SchemaManager::normalize_col_type("BLOB"), "BLOB");
        assert_eq!(SchemaManager::normalize_col_type("REAL"), "REAL");
    }

    #[tokio::test]
    async fn test_ensure_table_creates_table() {
        let pool = create_test_pool().await;

        let schema = vec![
            ColumnDef {
                name: "id".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: true,
                not_null: true,
            },
            ColumnDef {
                name: "name".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
        ];

        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();

        // Verify table exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)"
        )
        .bind("test_table")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(exists);
    }

    #[tokio::test]
    async fn test_ensure_table_idempotent() {
        let pool = create_test_pool().await;

        let schema = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            primary_key: true,
            not_null: true,
        }];

        // Create table twice
        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();
        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();

        // Verify table still exists and no error occurred
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)"
        )
        .bind("test_table")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(exists);
    }

    #[tokio::test]
    async fn test_ensure_indexes_created() {
        let pool = create_test_pool().await;

        let schema = vec![
            ColumnDef {
                name: "id".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: true,
                not_null: true,
            },
            ColumnDef {
                name: "name".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "value".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: false,
                not_null: false,
            },
        ];

        let indexes = vec![
            IndexDef {
                name: "idx_name".to_string(),
                columns: vec!["name".to_string()],
                unique: false,
            },
            IndexDef {
                name: "idx_unique_value".to_string(),
                columns: vec!["value".to_string()],
                unique: true,
            },
        ];

        SchemaManager::ensure_table(&pool, "test_table", &schema, &indexes)
            .await
            .unwrap();

        // Verify indexes exist
        let idx_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='test_table' AND name IN ('idx_name', 'idx_unique_value')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(idx_count, 2);
    }

    #[tokio::test]
    async fn test_schema_with_all_column_types() {
        let pool = create_test_pool().await;

        let schema = vec![
            ColumnDef {
                name: "int_col".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "text_col".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "blob_col".to_string(),
                col_type: "BLOB".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "real_col".to_string(),
                col_type: "REAL".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "json_col".to_string(),
                col_type: "JSON".to_string(), // Should be normalized to TEXT
                primary_key: false,
                not_null: false,
            },
        ];

        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();

        // Query table_info to verify column types
        let columns: Vec<(String, String)> = sqlx::query_as(
            "SELECT name, type FROM pragma_table_info('test_table')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        // JSON should be normalized to TEXT
        let json_col = columns.iter().find(|(name, _)| name == "json_col");
        assert_eq!(json_col.unwrap().1, "TEXT");
    }

    #[tokio::test]
    async fn test_schema_with_composite_primary_key() {
        let pool = create_test_pool().await;

        let schema = vec![
            ColumnDef {
                name: "id1".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: true,
                not_null: true,
            },
            ColumnDef {
                name: "id2".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: true,
                not_null: true,
            },
            ColumnDef {
                name: "data".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
        ];

        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();

        // Verify table was created
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)"
        )
        .bind("test_table")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(exists);
    }

    #[tokio::test]
    async fn test_schema_with_no_primary_key() {
        let pool = create_test_pool().await;

        let schema = vec![
            ColumnDef {
                name: "col1".to_string(),
                col_type: "INTEGER".to_string(),
                primary_key: false,
                not_null: false,
            },
            ColumnDef {
                name: "col2".to_string(),
                col_type: "TEXT".to_string(),
                primary_key: false,
                not_null: false,
            },
        ];

        SchemaManager::ensure_table(&pool, "test_table", &schema, &[])
            .await
            .unwrap();

        // Verify table was created without PK
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)"
        )
        .bind("test_table")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(exists);
    }
}
