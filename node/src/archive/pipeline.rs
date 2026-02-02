// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Core pipeline trait and context types for ETL indexing.

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use node_data::events::contract::ContractEvent;
use sqlx::SqlitePool;

use crate::archive::transformer::EventIdentifier;

/// Context provided to pipelines for each finalized block.
#[derive(Debug, Clone)]
pub struct PipelineContext<'a> {
    /// The height of the finalized block.
    pub block_height: u64,

    /// The hash of the finalized block (hex-encoded).
    pub block_hash: &'a str,

    /// Events grouped by transaction origin.
    pub grouped_events: &'a BTreeMap<EventIdentifier, Vec<ContractEvent>>,

    /// Whether any phoenix events were present in this block.
    #[allow(dead_code)]
    pub phoenix_present: bool,
}

/// Statistics returned after a pipeline run.
#[derive(Debug, Clone, Default)]
pub struct PipelineRunStats {
    /// Number of rows inserted/updated.
    pub rows_affected: u64,

    /// Number of events processed.
    pub events_processed: u64,

    /// Number of decode failures (if applicable).
    pub decode_failures: u64,
}

/// A pipeline processes finalized block data and produces derived outputs.
#[async_trait]
pub trait Pipeline: Send + Sync {
    /// Unique identifier for this pipeline instance.
    fn id(&self) -> &str;

    /// Pipeline type (e.g., "moonlight_builtin", "sql_event_table").
    fn pipeline_type(&self) -> &'static str;

    /// Ensure the pipeline's storage schema exists.
    ///
    /// Called once at startup after base migrations.
    async fn ensure_schema(&self, writer: &SqlitePool) -> Result<()>;

    /// Run the pipeline for a finalized block.
    ///
    /// This is called after canonical finalization is committed.
    async fn run_for_block(
        &self,
        writer: &SqlitePool,
        ctx: &PipelineContext<'_>,
    ) -> Result<PipelineRunStats>;
}
