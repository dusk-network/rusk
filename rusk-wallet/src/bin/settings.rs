// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::path::PathBuf;

use rusk_wallet::{Error, RuesHttpClient};
use tracing::Level;
use url::Url;

use crate::config::Network;
use crate::io::WalletArgs;

#[derive(clap::ValueEnum, Debug, Clone)]
pub(crate) enum LogFormat {
    Json,
    Plain,
    Coloured,
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub(crate) enum LogLevel {
    /// Designates very low priority, often extremely verbose, information.
    Trace,
    /// Designates lower priority information.
    Debug,
    /// Designates useful information.
    Info,
    /// Designates hazardous situations.
    Warn,
    /// Designates very serious errors.
    Error,
}

#[derive(Debug)]
pub(crate) struct Logging {
    /// Max log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Settings {
    pub(crate) state: Url,
    pub(crate) prover: Url,
    pub(crate) explorer: Option<Url>,

    pub(crate) logging: Logging,

    pub(crate) wallet_dir: PathBuf,
    pub(crate) password: Option<String>,
}

pub(crate) struct SettingsBuilder {
    wallet_dir: PathBuf,
    pub(crate) args: WalletArgs,
}

impl SettingsBuilder {
    pub fn wallet_dir(&self) -> &PathBuf {
        &self.wallet_dir
    }

    pub fn network(self, network: Network) -> Result<Settings, Error> {
        let args = self.args;

        let network = match (args.network, network.clone().network) {
            (Some(label), Some(mut networks)) => {
                let r = networks.remove(&label);
                // err if specified network is not in the list
                if r.is_none() {
                    return Err(Error::BadAddress);
                }

                r
            }
            // err if no networks are specified but argument is
            (Some(_), None) => {
                return Err(Error::BadAddress);
            }
            (_, _) => None,
        }
        .unwrap_or(network);

        let state = args
            .state
            .as_ref()
            .and_then(|value| Url::parse(value).ok())
            .unwrap_or(network.state);

        let prover = args
            .prover
            .as_ref()
            .and_then(|value| Url::parse(value).ok())
            .unwrap_or(network.prover);

        let explorer = network.explorer;

        let wallet_dir =
            args.wallet_dir.as_ref().cloned().unwrap_or(self.wallet_dir);

        let password = args.password;

        let logging = Logging {
            level: args.log_level,
            format: args.log_type,
        };

        Ok(Settings {
            state,
            prover,
            explorer,
            logging,
            wallet_dir,
            password,
        })
    }
}

impl Settings {
    pub fn args(args: WalletArgs) -> Result<SettingsBuilder, Error> {
        let wallet_dir = if let Some(path) = &args.wallet_dir {
            path.clone()
        } else {
            let mut path = dirs::home_dir().ok_or(Error::OsNotSupported)?;
            path.push(".dusk");
            path.push(env!("CARGO_BIN_NAME"));
            path
        };

        Ok(SettingsBuilder { wallet_dir, args })
    }

    pub async fn check_state_con(&self) -> Result<(), Error> {
        RuesHttpClient::new(self.state.as_ref())?
            .check_connection()
            .await
            .map_err(Error::from)
    }

    pub async fn check_prover_con(&self) -> Result<(), Error> {
        RuesHttpClient::new(self.prover.as_ref())?
            .check_connection()
            .await
            .map_err(Error::from)
    }
}

impl From<&LogLevel> for Level {
    fn from(level: &LogLevel) -> Level {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl fmt::Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Json => "json",
                Self::Plain => "plain",
                Self::Coloured => "coloured",
            }
        )
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Trace => "trace",
                Self::Debug => "debug",
                Self::Info => "info",
                Self::Warn => "warn",
                Self::Error => "error",
            }
        )
    }
}

impl fmt::Display for Logging {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Logging: [{}] ({})", self.level, self.format)
    }
}

impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = "─".repeat(14);
        writeln!(f, "{separator}")?;
        writeln!(f, "Settings")?;
        writeln!(f, "{separator}")?;
        writeln!(f, "Wallet directory: {}", self.wallet_dir.display())?;
        writeln!(
            f,
            "Password: {}",
            if self.password.is_some() {
                "[Set]"
            } else {
                "[Not set]"
            }
        )?;
        writeln!(f, "{}", separator)?;
        writeln!(f, "state: {}", self.state)?;
        writeln!(f, "prover: {}", self.prover)?;

        if let Some(explorer) = &self.explorer {
            writeln!(f, "explorer: {explorer}")?;
        }

        writeln!(f, "{separator}")?;
        writeln!(f, "{}", self.logging)
    }
}
