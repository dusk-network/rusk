// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use node::database::DatabaseOptions;
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct ChainConfig {
    db_path: Option<PathBuf>,
    db_options: Option<DatabaseOptions>,

    consensus_keys_path: Option<PathBuf>,
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    generation_timeout: Option<Duration>,

    max_queue_size: Option<usize>,

    // NB: changing the gas_per_deploy_byte/block_gas_limit is equivalent to
    // forking the chain.
    gas_per_deploy_byte: Option<u64>,
    min_deployment_gas_price: Option<u64>,
    min_deploy_points: Option<u64>,
    min_gas_limit: Option<u64>,
    block_gas_limit: Option<u64>,
    public_sender_start_height: Option<u64>,

    #[serde(with = "humantime_serde")]
    #[serde(default)]
    genesis_timestamp: Option<SystemTime>,
}

impl ChainConfig {
    pub(crate) fn merge(&mut self, args: &Args) {
        // Overwrite config consensus-keys-path
        if let Some(consensus_keys_path) = args.consensus_keys_path.clone() {
            self.consensus_keys_path = Some(consensus_keys_path);
        }

        // Overwrite config db-path
        if let Some(db_path) = args.db_path.clone() {
            self.db_path = Some(db_path);
        }
    }

    pub(crate) fn db_path(&self) -> PathBuf {
        self.db_path.clone().unwrap_or_else(|| {
            let mut path = dirs::home_dir().expect("OS not supported");
            path.push(".dusk");
            path.push(env!("CARGO_BIN_NAME"));
            path
        })
    }

    pub(crate) fn consensus_keys_path(&self) -> String {
        self.consensus_keys_path
            .clone()
            .unwrap_or_else(|| {
                let mut path = dirs::home_dir().expect("OS not supported");
                path.push(".dusk");
                path.push(env!("CARGO_BIN_NAME"));
                path.push("consensus.keys");
                path
            })
            .as_path()
            .display()
            .to_string()
    }

    pub(crate) fn db_options(&self) -> DatabaseOptions {
        self.db_options.clone().unwrap_or_default()
    }

    pub(crate) fn generation_timeout(&self) -> Option<Duration> {
        self.generation_timeout
    }

    pub(crate) fn gas_per_deploy_byte(&self) -> Option<u64> {
        self.gas_per_deploy_byte
    }

    pub(crate) fn min_deployment_gas_price(&self) -> Option<u64> {
        self.min_deployment_gas_price
    }

    pub(crate) fn min_deploy_points(&self) -> Option<u64> {
        self.min_deploy_points
    }

    pub(crate) fn public_sender_start_height(&self) -> Option<u64> {
        self.public_sender_start_height
    }

    pub(crate) fn min_gas_limit(&self) -> Option<u64> {
        self.min_gas_limit
    }

    pub(crate) fn max_queue_size(&self) -> usize {
        self.max_queue_size.unwrap_or(10_000)
    }

    pub(crate) fn block_gas_limit(&self) -> Option<u64> {
        self.block_gas_limit
    }

    pub(crate) fn genesis_timestamp(&self) -> u64 {
        self.genesis_timestamp
            .map(|t| {
                t.duration_since(UNIX_EPOCH)
                    .map(|n| n.as_secs())
                    .expect("This is heavy.")
            })
            .unwrap_or_default()
    }
}
