// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct ChainConfig {
    db_path: Option<PathBuf>,
    consensus_keys_path: Option<PathBuf>,
}

impl ChainConfig {
    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        // Overwrite config consensus-keys-path
        if let Some(consensus_keys_path) =
            matches.get_one::<String>("consensus-keys-path")
        {
            self.consensus_keys_path = Some(PathBuf::from(consensus_keys_path));
        }

        // Overwrite config db-path
        if let Some(db_path) = matches.get_one::<String>("db-path") {
            self.db_path = Some(PathBuf::from(db_path));
        }
    }

    pub fn inject_args(command: Command) -> Command {
        command
            .arg(
                Arg::new("consensus-keys-path")
                    .long("consensus-keys-path")
                    .value_name("CONSENSUS_KEYS_PATH")
                    .help("path to encrypted BLS keys")
                    .num_args(1),
            )
            .arg(
                Arg::new("db-path")
                    .long("db-path")
                    .value_name("DB_PATH")
                    .help("path to blockchain database")
                    .num_args(1),
            )
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
}
