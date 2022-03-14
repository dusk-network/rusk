// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{env, fs, path::PathBuf};

use crate::{LocalStore, WalletArgs};

/// Default Rusk IP address
pub(crate) const RUSK_ADDR: &str = "127.0.0.1";
/// Default Rusk TCP port
pub(crate) const RUSK_PORT: &str = "8585";
/// Default UDS path that Rusk GRPC-server will connect to
pub(crate) const RUSK_SOCKET: &str = "/tmp/rusk_listener";

mod parser {

    use crate::Error;
    use serde::Deserialize;
    use std::path::PathBuf;

    #[derive(Deserialize)]
    pub struct ParsedConfig {
        pub wallet: ParsedWalletConfig,
        pub rusk: ParsedRuskConfig,
        pub explorer: ParsedExplorerConfig,
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
        pub rusk_port: Option<String>,
        pub prover_addr: Option<String>,
        pub prover_port: Option<String>,
        pub socket_path: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct ParsedExplorerConfig {
        pub tx_url: Option<String>,
    }

    /// Attempts to parse the content of a file into config values
    pub fn parse(content: &str) -> Result<ParsedConfig, Error> {
        toml::from_str(content).map_err(Error::TOML)
    }
}

/// Config holds the settings for the CLI wallet
#[derive(Debug)]
pub(crate) struct Config {
    /// Wallet configuration
    pub wallet: WalletConfig,
    /// Rusk connection configuration
    pub rusk: RuskConfig,
    /// Dusk explorer configuration
    pub explorer: ExplorerConfig,
}

/// Wallet and store configuration
#[derive(Debug)]
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
#[derive(Debug)]
pub(crate) struct RuskConfig {
    /// IPC method for communication with rusk
    pub ipc_method: String,
    /// Rusk address
    pub rusk_addr: String,
    /// Rusk port
    pub rusk_port: String,
    /// Prover service address
    pub prover_addr: String,
    /// Prover service port
    pub prover_port: String,
    /// Path for setting up the unix domain socket
    pub socket_path: String,
}

/// Dusk Explorer access information
#[derive(Debug)]
pub(crate) struct ExplorerConfig {
    /// Base url for transactions
    pub tx_url: Option<String>,
}

impl Config {
    /// Attempt to load configuration from file
    pub fn load() -> Option<Config> {
        // search in default data dir and current working directory
        let mut paths =
            vec![env::current_dir().ok()?, LocalStore::default_data_dir()];

        for p in paths.iter_mut() {
            // file is always called "config.toml"
            p.push("config");
            p.set_extension("toml");
            println!("{}", p.display());

            // attempt to read the file
            if let Ok(content) = fs::read_to_string(&p) {
                match parser::parse(content.as_str()) {
                    Ok(loaded) => {
                        println!("Using config from {}", p.display());
                        return Some(loaded.into());
                    }
                    Err(err) => {
                        println!("Failed to read {}:\n{}", p.display(), err);
                    }
                }
            }
        }

        None
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
            self.rusk.rusk_addr = rusk_addr.clone();
            self.rusk.prover_addr = rusk_addr;
        }
        if let Some(rusk_port) = args.rusk_port {
            self.rusk.rusk_port = rusk_port.clone();
            self.rusk.prover_port = rusk_port;
        }
        if let Some(prover_addr) = args.prover_addr {
            self.rusk.prover_addr = prover_addr;
        }
        if let Some(prover_port) = args.prover_port {
            self.rusk.prover_port = prover_port;
        }
        if let Some(socket_path) = args.socket_path {
            self.rusk.socket_path = socket_path;
        }
        if let Some(skip_recovery) = args.skip_recovery {
            self.wallet.skip_recovery = skip_recovery;
        }
    }

    /// Default settings
    pub fn default() -> Config {
        Config {
            wallet: WalletConfig {
                data_dir: LocalStore::default_data_dir(),
                name: LocalStore::default_wallet_name(),
                file: None,
                skip_recovery: false,
            },
            rusk: RuskConfig {
                ipc_method: "uds".to_string(),
                rusk_addr: RUSK_ADDR.to_string(),
                rusk_port: RUSK_PORT.to_string(),
                prover_addr: RUSK_ADDR.to_string(),
                prover_port: RUSK_PORT.to_string(),
                socket_path: RUSK_SOCKET.to_string(),
            },
            explorer: ExplorerConfig { tx_url: None },
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
                    .unwrap_or_else(|| "uds".to_string()),
                rusk_addr: parsed
                    .rusk
                    .rusk_addr
                    .unwrap_or_else(|| RUSK_ADDR.to_string()),
                rusk_port: parsed
                    .rusk
                    .rusk_port
                    .unwrap_or_else(|| RUSK_PORT.to_string()),
                prover_addr: parsed
                    .rusk
                    .prover_addr
                    .unwrap_or_else(|| RUSK_ADDR.to_string()),
                prover_port: parsed
                    .rusk
                    .prover_port
                    .unwrap_or_else(|| RUSK_PORT.to_string()),
                socket_path: parsed
                    .rusk
                    .socket_path
                    .unwrap_or_else(|| RUSK_SOCKET.to_string()),
            },
            explorer: ExplorerConfig {
                tx_url: parsed.explorer.tx_url,
            },
        }
    }
}
