// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct ChainConfig {
    db_path: Option<PathBuf>,
    consensus_keys_path: Option<PathBuf>,
    #[serde(with = "humantime_serde")]
    generation_timeout: Option<Duration>,
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

    pub(crate) fn generation_timeout(&self) -> Option<Duration> {
        self.generation_timeout
    }
}
