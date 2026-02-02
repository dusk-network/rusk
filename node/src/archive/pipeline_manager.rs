// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Pipeline manager orchestrates ETL pipeline execution.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::{debug, error, info, warn};

use crate::archive::pipeline::{Pipeline, PipelineContext};
use crate::archive::conf::pipeline_config::{PipelineConfig, PipelinesConfig};
use crate::archive::schema_manager::ensure_pipeline_meta_table;

/// Manages pipeline lifecycle and execution.
pub struct PipelineManager {
    /// Registered pipelines by ID.
    pipelines: HashMap<String, Arc<dyn Pipeline>>,

    /// Original config for reference.
    config: PipelinesConfig,
}

impl PipelineManager {
    /// Create a new pipeline manager from configuration.
    pub fn new(config: PipelinesConfig) -> Self {
        Self {
            pipelines: HashMap::new(),
            config,
        }
    }

    /// Register a pipeline implementation.
    pub fn register(&mut self, pipeline: Arc<dyn Pipeline>) {
        let id = pipeline.id().to_string();
        debug!("Registering pipeline: {}", id);
        self.pipelines.insert(id, pipeline);
    }

    /// Initialize all pipelines (schema setup).
    pub async fn initialize(&self, writer: &SqlitePool) -> Result<()> {
        info!("Initializing pipeline manager...");

        // Ensure pipeline_meta table exists
        ensure_pipeline_meta_table(writer).await?;

        // Initialize each enabled pipeline
        for pipeline_cfg in &self.config.pipelines {
            if !pipeline_cfg.enabled {
                info!("Pipeline '{}' is disabled, skipping", pipeline_cfg.id);
                continue;
            }

            let pipeline =
                self.pipelines.get(&pipeline_cfg.id).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Pipeline '{}' is enabled in config but not registered",
                        pipeline_cfg.id
                    )
                })?;

            info!(
                "Initializing pipeline '{}' (type: {})",
                pipeline.id(),
                pipeline.pipeline_type()
            );

            // Ensure schema
            pipeline.ensure_schema(writer).await.with_context(|| {
                format!(
                    "Failed to ensure schema for pipeline '{}'",
                    pipeline.id()
                )
            })?;

            // Upsert pipeline_meta
            self.upsert_pipeline_meta(writer, pipeline_cfg, pipeline.as_ref())
                .await?;

            info!("Pipeline '{}' initialized successfully", pipeline.id());
        }

        info!(
            "Pipeline manager initialized with {} active pipelines",
            self.config.pipelines.iter().filter(|p| p.enabled).count()
        );

        Ok(())
    }

    /// Run all enabled pipelines for a finalized block.
    pub async fn run_for_block(
        &self,
        writer: &SqlitePool,
        ctx: &PipelineContext<'_>,
    ) -> Result<()> {
        debug!(
            "Running pipelines for block {} (height {})",
            ctx.block_hash, ctx.block_height
        );

        for pipeline_cfg in &self.config.pipelines {
            if !pipeline_cfg.enabled {
                continue;
            }

            let pipeline = match self.pipelines.get(&pipeline_cfg.id) {
                Some(p) => p,
                None => {
                    warn!(
                        "Pipeline '{}' is enabled but not registered, skipping",
                        pipeline_cfg.id
                    );
                    continue;
                }
            };

            // Run pipeline
            let result = pipeline.run_for_block(writer, ctx).await;

            match result {
                Ok(stats) => {
                    debug!(
                        "Pipeline '{}' completed: {} rows affected, {} events processed",
                        pipeline.id(), stats.rows_affected, stats.events_processed
                    );

                    // Update checkpoint
                    self.update_checkpoint(
                        writer,
                        &pipeline_cfg.id,
                        ctx.block_height,
                        None,
                    )
                    .await?;
                }
                Err(e) => {
                    error!("Pipeline '{}' failed: {}", pipeline.id(), e);

                    // Record error
                    self.update_checkpoint(
                        writer,
                        &pipeline_cfg.id,
                        ctx.block_height,
                        Some(&format!("{:#}", e)),
                    )
                    .await?;

                    // If required, propagate error
                    if pipeline_cfg.required {
                        return Err(e).context(format!(
                            "Required pipeline '{}' failed",
                            pipeline.id()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Upsert pipeline metadata on initialization.
    async fn upsert_pipeline_meta(
        &self,
        pool: &SqlitePool,
        cfg: &PipelineConfig,
        _pipeline: &dyn Pipeline,
    ) -> Result<()> {
        let config_json = serde_json::to_string(cfg)?;
        let config_hash = {
            let hash = blake2b_simd::blake2b(config_json.as_bytes());
            hex::encode(&hash.as_bytes()[..16]) // Use first 16 bytes like md5
        };

        let schema_json = serde_json::to_string(&cfg.sink)?;
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        sqlx::query(
            r#"
            INSERT INTO pipeline_meta (pipeline_id, pipeline_type, config_hash, schema_json, created_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(pipeline_id) DO UPDATE SET
                pipeline_type = excluded.pipeline_type,
                config_hash = excluded.config_hash,
                schema_json = excluded.schema_json
            "#,
        )
        .bind(&cfg.id)
        .bind(&cfg.pipeline_type)
        .bind(&config_hash)
        .bind(&schema_json)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update pipeline checkpoint after processing a block.
    async fn update_checkpoint(
        &self,
        pool: &SqlitePool,
        id: &str,
        height: u64,
        error: Option<&str>,
    ) -> Result<()> {
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        match error {
            Some(e) => {
                sqlx::query("UPDATE pipeline_meta SET last_error = ?1, last_error_at = ?2 WHERE pipeline_id = ?3")
                    .bind(e).bind(now).bind(id).execute(pool).await?;
            }
            None => {
                sqlx::query("UPDATE pipeline_meta SET last_processed_height = ?1, last_error = NULL, last_error_at = NULL WHERE pipeline_id = ?2")
                    .bind(height as i64).bind(id).execute(pool).await?;
            }
        }
        Ok(())
    }

    /// Get the list of enabled pipeline IDs.
    pub fn enabled_pipeline_ids(&self) -> Vec<&str> {
        self.config
            .pipelines
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.id.as_str())
            .collect()
    }
}
