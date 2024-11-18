// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use url::Url;

use crate::Error;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct Network {
    pub(crate) state: Url,
    pub(crate) prover: Url,
    pub(crate) explorer: Option<Url>,
    pub(crate) network: Option<HashMap<String, Network>>,
}

use std::{fs, io};

/// Config holds the settings for the CLI wallet
#[derive(Debug)]
pub struct Config {
    /// Network configuration
    pub(crate) network: Network,
}

fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<Option<String>> {
    fs::read_to_string(&path)
        .map(Some)
        .or_else(|e| match e.kind() {
            io::ErrorKind::NotFound => Ok(None),
            _ => Err(e),
        })
}

impl Config {
    /// Attempt to load configuration from file
    pub fn load(profile: &Path) -> Result<Config, Error> {
        let profile = profile.join("config.toml");

        let mut global_config = dirs::home_dir();

        match global_config {
            Some(ref mut path) => {
                path.push(".config");
                path.push(env!("CARGO_BIN_NAME"));
                path.push("config.toml");

                let contents = read_to_string(profile)?
                    .or(read_to_string(&path)?)
                    .unwrap_or_else(|| {
                        include_str!("../../default.config.toml").to_string()
                    });

                let network: Network = toml::from_str(&contents)
                    .map_err(|_| Error::NetworkNotFound)?;

                Ok(Config { network })
            }
            None => Err(Error::OsNotSupported),
        }
    }
}
