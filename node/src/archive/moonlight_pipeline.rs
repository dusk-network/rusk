// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Built-in Moonlight pipeline implementation.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::archive::pipeline::{Pipeline, PipelineContext, PipelineRunStats};
use crate::archive::Archive;

/// Built-in pipeline for Moonlight transfer indexing.
///
/// This wraps the existing Moonlight indexer (tl_moonlight +
/// update_active_accounts) as a pipeline implementation.
pub struct MoonlightPipeline {
    /// Reference to the archive (for accessing moonlight_db and methods).
    archive: Arc<Archive>,
}

impl MoonlightPipeline {
    pub fn new(archive: Arc<Archive>) -> Self {
        Self { archive }
    }
}

#[async_trait]
impl Pipeline for MoonlightPipeline {
    fn id(&self) -> &str {
        "moonlight_builtin"
    }

    fn pipeline_type(&self) -> &'static str {
        "moonlight_builtin"
    }

    async fn ensure_schema(&self, _writer: &SqlitePool) -> Result<()> {
        // Moonlight schema is managed separately via RocksDB + active_accounts
        // table The active_accounts table already exists from
        // migrations
        Ok(())
    }

    async fn run_for_block(
        &self,
        _writer: &SqlitePool,
        ctx: &PipelineContext<'_>,
    ) -> Result<PipelineRunStats> {
        // Run the existing Moonlight transform+load
        let active_accounts =
            self.archive.tl_moonlight(ctx.grouped_events.clone())?;

        // Update active accounts in SQLite
        let rows_affected =
            self.archive.update_active_accounts(active_accounts).await?;

        Ok(PipelineRunStats {
            rows_affected,
            events_processed: ctx.grouped_events.len() as u64,
            decode_failures: 0,
        })
    }
}
