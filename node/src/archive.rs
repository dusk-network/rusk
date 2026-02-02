// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rocksdb::OptimisticTransactionDB;
use sqlx::sqlite::SqlitePool;
use tracing::debug;

use crate::archive::conf::pipeline_config::PipelinesConfig;

pub mod conf;
mod moonlight;
mod moonlight_pipeline;
mod pipeline;
mod pipeline_manager;
mod schema_manager;
mod sql_event_table_pipeline;
mod sqlite;
mod transformer;

/// etl unit tests module.
#[cfg(test)]
mod etl_pipeline_tests;

use conf::Params as ArchiveParams;
use pipeline_manager::PipelineManager;

pub use moonlight::{MoonlightGroup, Order};

// Archive folder containing the sqlite database and the moonlight database
const ARCHIVE_FOLDER_NAME: &str = "archive";

/// The Archive for the archive node.
///
/// The implementation for the sqlite archive and archivist trait is in the
/// `sqlite` module. The implementation for the moonlight database is in the
/// `moonlight` module.
#[derive(Clone)]
pub struct Archive {
    // Writer pool (single connection) to the SQLite database
    sqlite_writer: SqlitePool,
    // Reader pool (read-only) for GraphQL/queries
    sqlite_reader: SqlitePool,
    // The connection pool can be cloned and stays the same as it is behind an
    // Arc PoolInner. Pool<DB: Database>(pub(crate) Arc<PoolInner<DB>>)
    // The moonlight database.
    moonlight_db: Arc<OptimisticTransactionDB>,
    // last finalized block height known to the archive
    last_finalized_block_height: u64,
    // Pipeline manager for ETL indexing
    pipeline_manager: Option<Arc<PipelineManager>>,
}

impl Archive {
    fn archive_folder_path<P: AsRef<Path>>(base_path: P) -> PathBuf {
        let path = base_path.as_ref().join(ARCHIVE_FOLDER_NAME);

        // Recursively create the archive folder if it doesn't exist already
        fs::create_dir_all(&path).unwrap_or_else(|_| {
            panic!("creating directory in {path:?} should not fail")
        });
        path
    }

    /// Create or open the archive database
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path to the base folder where the archive folder
    ///   resides in or will be created.
    pub async fn create_or_open<P: AsRef<Path>>(base_path: P) -> Self {
        Self::create_or_open_with_conf(base_path, ArchiveParams::default())
            .await
    }

    /// Create or open the archive database with configuration parameters
    ///
    /// # Arguments
    ///
    /// * `base_path` - The path to the base folder where the archive folder
    ///   resides in or will be created.
    /// * `params` - Archive node configuration parameters.
    pub async fn create_or_open_with_conf<P: AsRef<Path>>(
        base_path: P,
        params: ArchiveParams,
    ) -> Self {
        let path = Self::archive_folder_path(base_path);

        tracing::info!(
            "Archive::create_or_open_with_conf with conf {}",
            params
        );

        let sqlite_writer = Self::create_writer_pool(&path).await;
        let sqlite_reader =
            Self::create_reader_pool(&path, params.reader_max_connections)
                .await;
        let moonlight_db =
            Self::create_or_open_moonlight_db(&path, params.clone());

        let mut self_archive = Self {
            sqlite_writer: sqlite_writer.clone(),
            sqlite_reader,
            moonlight_db,
            last_finalized_block_height: 0,
            pipeline_manager: None,
        };

        let last_finalized_block_height = match self_archive
            .fetch_last_finalized_block()
            .await
        {
            Ok((height, _)) => height,
            Err(e) => {
                // If the error is sqlx::Error::RowNotFound then it is fine
                // during sync from scratch
                debug!("Error fetching last finalized block height during archive initialization: {e}");
                0
            }
        };

        self_archive.last_finalized_block_height = last_finalized_block_height;

        // Load and initialize pipeline manager if config provided
        if let Some(config_path) = params.pipelines_config_path {
            match PipelinesConfig::load(&config_path) {
                Ok(config) => {
                    tracing::info!(
                        "Loaded pipelines config from {:?}",
                        config_path
                    );

                    let mut manager = PipelineManager::new(config.clone());

                    // Register pipelines based on config
                    for pipeline_cfg in &config.pipelines {
                        if !pipeline_cfg.enabled {
                            continue;
                        }

                        match pipeline_cfg.pipeline_type.as_str() {
                            "moonlight_builtin" => {
                                let pipeline = Arc::new(
                                    moonlight_pipeline::MoonlightPipeline::new(Arc::new(self_archive.clone()))
                                );
                                manager.register(pipeline);
                            }
                            "sql_event_table" => {
                                match sql_event_table_pipeline::SqlEventTablePipeline::new(pipeline_cfg.clone()) {
                                    Ok(pipeline) => {
                                        manager.register(Arc::new(pipeline));
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to create sql_event_table pipeline '{}': {}", pipeline_cfg.id, e);
                                        panic!("Pipeline creation failed: {}", e);
                                    }
                                }
                            }
                            other => {
                                tracing::error!("Unknown pipeline type: {}", other);
                                panic!("Unknown pipeline type: {}", other);
                            }
                        }
                    }

                    // Initialize the pipeline manager (creates tables and
                    // metadata)
                    if let Err(e) =
                        manager.initialize(&self_archive.sqlite_writer).await
                    {
                        tracing::error!(
                            "Failed to initialize pipeline manager: {}",
                            e
                        );
                        panic!("Pipeline manager initialization failed: {}", e);
                    }

                    tracing::info!(
                        "Initialized {} pipelines",
                        manager.enabled_pipeline_ids().len()
                    );

                    self_archive.pipeline_manager = Some(Arc::new(manager));
                }
                Err(e) => {
                    tracing::error!("Failed to load pipelines config: {}", e);
                    panic!("Pipelines config loading failed: {}", e);
                }
            }
        } else {
            tracing::info!("No pipelines config provided, pipelines disabled");
        }

        self_archive
    }

    /// Returns the last finalized block height cached in the archive.
    ///
    /// # Note
    /// This is defined as the last finalized block height that was stored in
    /// the archive. This means also that it is the last height that the
    /// specific archive is aware of.
    pub fn last_finalized_block_height(&self) -> u64 {
        self.last_finalized_block_height
    }
}
