// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Formatter;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(crate) mod pipeline_config;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    /// Max write buffer size for moonlight event CF.
    pub events_cf_max_write_buffer_size: usize,

    /// Block Cache is useful in optimizing DB reads.
    pub events_cf_disable_block_cache: bool,

    /// Enables a set of flags for collecting DB stats as log data.
    pub enable_debug: bool,

    /// Max number of connections in the SQLite reader pool.
    pub reader_max_connections: u32,

    /// Optional path to pipelines configuration JSON file.
    /// If None, no ETL pipelines will be executed (except built-in defaults).
    pub pipelines_config_path: Option<PathBuf>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            events_cf_max_write_buffer_size: 1024 * 1024, // 1 MiB
            events_cf_disable_block_cache: false,
            enable_debug: false,
            reader_max_connections: 16,
            pipelines_config_path: None,
        }
    }
}

impl std::fmt::Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "events_cf_max_write_buffer_size: {}, \
             events_cf_disable_block_cache: {}, \
             enable_debug: {}, \
             reader_max_connections: {}, \
             pipelines_config_path: {:?}",
            self.events_cf_max_write_buffer_size,
            self.events_cf_disable_block_cache,
            self.enable_debug,
            self.reader_max_connections,
            self.pipelines_config_path,
        )
    }
}
