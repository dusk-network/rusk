// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, LocalStore, WalletArgs};
use serde::Serialize;
use std::{fs, io, path::PathBuf};

/// Default IPC method for Rusk connections
pub(crate) const IPC_DEFAULT: &str = "uds";
/// Default Rusk address uses UDS
pub(crate) const RUSK_ADDR: &str = "/tmp/rusk_listener";
/// Default GraphQL endopoint
pub(crate) const GQL_ADDR: &str = "http://nodes.dusk.network:9500/graphql";

mod parser {

    use crate::Error;
    use serde::Deserialize;
    use std::path::PathBuf;

    #[derive(Deserialize)]
    pub struct ParsedConfig {
        pub wallet: ParsedWalletConfig,
        pub rusk: ParsedRuskConfig,
        pub explorer: ParsedExplorerConfig,
        pub chain: ParsedChainConfig,
    }

    #[derive(Deserialize)]
    pub struct ParsedWalletConfig {
        pub data_dir: Option<PathBuf>,
        pub wallet_name: Option<String>,
        pub wallet_file: Option<PathBuf>,
        pub skip_recovery: Option<bool>,
    }

    #[derive(Deserialize)]
    pub struct ParsedRuskConfig {
        pub ipc_method: Option<String>,
        pub rusk_addr: Option<String>,
        pub prover_addr: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct ParsedExplorerConfig {
        pub tx_url: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct ParsedChainConfig {
        pub gql_url: Option<String>,
        pub wait_for_tx: Option<bool>,
    }

    /// Attempts to parse the content of a file into config values
    pub fn parse(content: &str) -> Result<ParsedConfig, Error> {
        toml::from_str(content).map_err(Error::ConfigRead)
    }
}

/// Config holds the settings for the CLI wallet
#[derive(Serialize)]
pub(crate) struct Config {
    /// Wallet configuration
    pub wallet: WalletConfig,
    /// Rusk connection configuration
    pub rusk: RuskConfig,
    /// Dusk explorer configuration
    pub explorer: ExplorerConfig,
    /// Dusk chain configuration
    pub chain: ChainConfig,
}

/// Wallet and store configuration
#[derive(Serialize)]
pub(crate) struct WalletConfig {
    /// Directory to store user data
    pub data_dir: PathBuf,
    /// Wallet file name
    pub name: String,
    /// Path to a wallet file. Overrides `data-dir` and `wallet-name`
    pub file: Option<PathBuf>,
    /// Skip wallet recovery phrase (useful for headless wallet creation)
    pub skip_recovery: bool,
}

/// Connection details to Rusk and Prover clusters
#[derive(Serialize)]
pub(crate) struct RuskConfig {
    /// IPC method for communication with rusk
    pub ipc_method: String,
    /// Rusk address
    pub rusk_addr: String,
    /// Prover service address
    pub prover_addr: String,
}

/// Dusk Explorer access information
#[derive(Serialize)]
pub(crate) struct ExplorerConfig {
    /// Base url for transactions
    pub tx_url: Option<String>,
}

/// Dusk Chain configuration
#[derive(Serialize)]
pub(crate) struct ChainConfig {
    /// GraphQL http endpoint
    pub gql_url: String,
    /// Wait for tx confirmation
    pub wait_for_tx: bool,
}

impl Config {
    /// Attempt to load configuration from file
    ///
    /// If file doesn't exist, will extract the default and write in the
    /// allocated dir
    pub fn load(data_dir: PathBuf) -> Result<Config, Error> {
        let file = data_dir.join("config").with_extension("toml");

        let contents = match fs::read_to_string(&file) {
            Ok(c) => Ok(c),

            // File not found - use default config
            Err(e) if matches!(e.kind(), io::ErrorKind::NotFound) => {
                let default = include_str!("../../config.toml");

                match fs::write(&file, &default) {
                    Ok(_) => println!("Default configuration generated: {}", file.display()),

                    Err(e) => println!("Default configuration generated; failed to write in {}: {}", file.display(), e),
                }

                Ok(default.to_string())
            }

            Err(e) => Err(e),
        }?;

        let mut config = parser::parse(&contents).map(Config::from)?;

        config.wallet.data_dir = data_dir;

        Ok(config)
    }

    /// Arguments that have been explicitly passed into this
    /// execution replace the static configuration
    pub fn merge(&mut self, args: WalletArgs) {
        if let Some(data_dir) = args.data_dir {
            self.wallet.data_dir = data_dir;
        }
        if let Some(wallet_name) = args.wallet_name {
            self.wallet.name = wallet_name;
        }
        if let Some(wallet_file) = args.wallet_file {
            self.wallet.file = Some(wallet_file);
        }
        if let Some(ipc_method) = args.ipc_method {
            self.rusk.ipc_method = ipc_method;
        }
        if let Some(rusk_addr) = args.rusk_addr {
            self.rusk.rusk_addr = rusk_addr
        }
        if let Some(prover_addr) = args.prover_addr {
            self.rusk.prover_addr = prover_addr;
        }
        if let Some(skip_recovery) = args.skip_recovery {
            self.wallet.skip_recovery = skip_recovery;
        }
        if let Some(wait_for_tx) = args.wait_for_tx {
            self.chain.wait_for_tx = wait_for_tx;
        }
    }
}

impl From<parser::ParsedConfig> for Config {
    fn from(parsed: parser::ParsedConfig) -> Self {
        Config {
            wallet: WalletConfig {
                data_dir: parsed
                    .wallet
                    .data_dir
                    .unwrap_or_else(LocalStore::default_data_dir),
                name: parsed
                    .wallet
                    .wallet_name
                    .unwrap_or_else(LocalStore::default_wallet_name),
                file: parsed.wallet.wallet_file,
                skip_recovery: parsed.wallet.skip_recovery.unwrap_or(false),
            },
            rusk: RuskConfig {
                ipc_method: parsed
                    .rusk
                    .ipc_method
                    .unwrap_or_else(|| IPC_DEFAULT.to_string()),
                rusk_addr: parsed
                    .rusk
                    .rusk_addr
                    .unwrap_or_else(|| RUSK_ADDR.to_string()),
                prover_addr: parsed
                    .rusk
                    .prover_addr
                    .unwrap_or_else(|| RUSK_ADDR.to_string()),
            },
            explorer: ExplorerConfig {
                tx_url: parsed.explorer.tx_url,
            },
            chain: ChainConfig {
                gql_url: parsed
                    .chain
                    .gql_url
                    .unwrap_or_else(|| GQL_ADDR.to_string()),
                wait_for_tx: parsed.chain.wait_for_tx.unwrap_or(false),
            },
        }
    }
}
