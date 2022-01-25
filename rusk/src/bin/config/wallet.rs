// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{App, Arg, ArgMatches};
use rusk::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct WalletConfig {
    pub(crate) path: String,
    password: Option<String>,
    password_path: Option<String>,
}

/// Default wallet path.
pub const WALLET_PATH: &str = "wallet.dat";

impl Default for WalletConfig {
    fn default() -> Self {
        WalletConfig {
            path: WALLET_PATH.to_string(),
            password: None,
            password_path: None,
        }
    }
}

impl WalletConfig {
    pub fn _password(&self) -> Result<String> {
        if let Some(password) = &self.password {
            return Ok(password.to_string());
        };
        if let Some(password_path) = &self.password_path {
            return Ok(fs::read_to_string(password_path)?);
        };
        panic!("No <password> nor <password_path> are provided>");
    }

    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        if let Some(path) = matches.value_of("wallet_path") {
            self.path = path.into();
        }
        if let Some(password) = matches.value_of("wallet_password") {
            self.password = Some(password.into());
        }
        if let Some(password_path) = matches.value_of("wallet_password_path") {
            self.password_path = Some(password_path.into());
        }
    }
    pub fn inject_args(app: App<'_>) -> App<'_> {
        app.arg(
            Arg::new("wallet_path")
                .long("wallet_path")
                .value_name("wallet_path")
                .help("Path of the wallet")
                .takes_value(true),
        )
        .arg(
            Arg::new("wallet_password")
                .long("wallet_password")
                .value_name("wallet_password")
                .help("Password of the wallet")
                .takes_value(true),
        )
        .arg(
            Arg::new("wallet_password_path")
                .long("wallet_password_path")
                .value_name("wallet_password_path")
                .help("Path of file which contains the password wallet (useful for Docker Images secrets")
                .takes_value(true),
        )
    }
}
