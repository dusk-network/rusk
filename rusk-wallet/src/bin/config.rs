// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use url::Url;

use crate::Error;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct Network {
    pub(crate) state: Url,
    pub(crate) prover: Url,
    pub(crate) archiver: Option<Url>,
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

                // Try to read profile config first, then global config
                let contents = match read_to_string(&profile)? {
                    Some(contents) => Some(contents),
                    None => read_to_string(path.clone())?,
                };

                let contents = match contents {
                    Some(contents) => contents,
                    None => {
                        // If no config exists anywhere, create one in the
                        // global location
                        let default_config =
                            include_str!("../../default.config.toml");

                        // Create the global config directory if it doesn't
                        // exist
                        if let Some(parent) = path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }

                        // Write the default config to the global location
                        // Ignore errors - if writing fails, we'll just use the
                        // embedded default
                        let _ = fs::write(path, default_config);

                        default_config.to_string()
                    }
                };

                let network: Network = toml::from_str(&contents)
                    .map_err(|_| Error::NetworkNotFound)?;

                Ok(Config { network })
            }
            None => Err(Error::OsNotSupported),
        }
    }
}
