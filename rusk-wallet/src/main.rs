// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(crate) mod rusk_proto {
    tonic::include_proto!("rusk");
}

mod lib;
pub use lib::error::{Error, ProverError, StateError, StoreError};

use clap::{AppSettings, Parser, Subcommand};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use lib::clients::{Prover, State};
use lib::crypto::MnemSeed;
use lib::prompt;
use lib::store::LocalStore;
use lib::wallet::CliWallet;

use rusk_proto::network_client::NetworkClient;
use rusk_proto::prover_client::ProverClient;
use rusk_proto::state_client::StateClient;

/// Default Rusk IP address
pub(crate) const RUSK_ADDR: &str = "127.0.0.1";
/// Default Rusk TCP port
pub(crate) const RUSK_PORT: &str = "8585";
/// Default UDS path that Rusk GRPC-server will connect to
pub(crate) const RUSK_SOCKET: &str = "/tmp/rusk_listener";

/// The CLI Wallet
#[derive(Parser)]
#[clap(version)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(about = "A user-friendly, reliable command line interface to the Dusk wallet!", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub(crate) struct WalletCfg {
    /// Directory to store user data [default: `$HOME/.dusk`]
    #[clap(short, long)]
    data_dir: Option<PathBuf>,

    /// Name for your wallet [default: `$(whoami)`]
    #[clap(short = 'n', long, value_name = "NAME")]
    wallet_name: Option<String>,

    /// Path to a wallet file. Overrides `data-dir` and `wallet-name`, useful
    /// when loading a wallet that's not in the default directory.
    #[clap(short = 'f', long, parse(from_os_str), value_name = "PATH")]
    wallet_file: Option<PathBuf>,

    /// Rusk address
    #[clap(short = 'a', long, default_value_t = RUSK_ADDR.to_string())]
    rusk_addr: String,

    /// Rusk port
    #[clap(short = 'p', long, default_value_t = RUSK_PORT.to_string())]
    rusk_port: String,

    /// Prover service address [default: `rusk_addr`]
    #[clap(long)]
    prover_addr: Option<String>,

    /// Prover service port [default: `rusk_port`]
    #[clap(long)]
    prover_port: Option<String>,

    /// IPC method for communication with rusk [uds, tcp_ip]
    #[clap(short = 'i', long, default_value = "uds")]
    ipc_method: String,

    /// Path for setting up the unix domain socket
    #[clap(short = 's', long, default_value_t = RUSK_SOCKET.to_string())]
    socket_path: String,

    /// Skip wallet recovery phrase (useful for headless wallet creation)
    #[clap(long)]
    skip_recovery: bool,

    /// Command
    #[clap(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Create a new wallet
    Create,

    /// Restore a lost wallet
    Restore,

    /// Check your current balance
    Balance {
        /// Key index
        #[clap(short, long)]
        key: u64,
    },

    /// Retrieve public spend key
    Address {
        /// Key index
        #[clap(short, long)]
        key: u64,
    },

    /// Send Dusk through the network
    Transfer {
        /// Key index from which to send Dusk
        #[clap(short, long)]
        key: u64,

        /// Receiver address
        #[clap(short, long)]
        rcvr: String,

        /// Amount of Dusk to send (in µDusk)
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: u64,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Start staking Dusk
    Stake {
        /// Key index from which to stake Dusk
        #[clap(short, long)]
        key: u64,

        /// Staking key to sign this stake
        #[clap(short, long)]
        stake_key: u64,

        /// Amount of Dusk to stake (in µDusk)
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: u64,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Check your stake
    StakeInfo {
        /// Staking key used to sign the stake
        #[clap(short, long)]
        key: u64,
    },

    /// Withdraw a key's stake
    WithdrawStake {
        /// Key index from which your Dusk was staked
        #[clap(short, long)]
        key: u64,

        /// Staking key index used for this stake
        #[clap(short, long)]
        stake_key: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: u64,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Export BLS provisioner key pair
    Export {
        /// Key index from which your Dusk was staked
        #[clap(short, long)]
        key: u64,

        /// Don't encrypt the output file
        #[clap(long)]
        plaintext: bool,
    },

    /// Run in interactive mode (default)
    Interactive,
}

impl CliCommand {
    fn uses_wallet(&self) -> bool {
        !matches!(*self, Self::Create | Self::Restore | Self::Interactive)
    }
}

/// Client connections to rusk Services
struct Rusk {
    network: NetworkClient<Channel>,
    state: StateClient<Channel>,
    prover: ProverClient<Channel>,
}

/// Connect to rusk services via TCP
async fn rusk_tcp(rusk_addr: String, prov_addr: String) -> Result<Rusk, Error> {
    Ok(Rusk {
        network: NetworkClient::connect(rusk_addr.clone()).await?,
        state: StateClient::connect(rusk_addr).await?,
        prover: ProverClient::connect(prov_addr).await?,
    })
}

/// Connect to rusk via UDS (Unix domain sockets)
async fn rusk_uds(socket_path: String) -> Result<Rusk, Error> {
    let channel = Endpoint::try_from("http://[::]:50051")
        .expect("parse address")
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = (&socket_path[..]).to_string();
            UnixStream::connect(path)
        }))
        .await?;

    Ok(Rusk {
        network: NetworkClient::new(channel.clone()),
        state: StateClient::new(channel.clone()),
        prover: ProverClient::new(channel),
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    if let Err(err) = exec().await {
        println!("{}", err);
    }
    Ok(())
}

async fn exec() -> Result<(), Error> {
    use CliCommand::*;

    // parse cli arguments
    let cfg: WalletCfg = WalletCfg::parse();

    // create directories
    let data_dir = cfg.data_dir.unwrap_or_else(LocalStore::default_data_dir);
    LocalStore::create_dir(&data_dir)?;

    // get command or default to interactive mode
    let cmd = cfg.command.unwrap_or(CliCommand::Interactive);

    // request auth for wallet (if required)
    let pwd = if cmd.uses_wallet() || cfg.wallet_file.is_some() {
        prompt::request_auth("Please enter wallet password")
    } else {
        blake3::hash("".as_bytes())
    };

    // prepare wallet path
    let mut path_override = false;
    let wallet_path = if let Some(p) = cfg.wallet_file {
        path_override = true;
        p.with_extension("dat")
    } else {
        let wallet_name = cfg
            .wallet_name
            .unwrap_or_else(LocalStore::default_wallet_name);
        let mut pb = PathBuf::new();
        pb.push(&data_dir);
        pb.push(&wallet_name);
        pb.set_extension("dat");
        pb
    };

    // start our local store
    let store = match cmd {
        Create => create(wallet_path, cfg.skip_recovery)?,
        Restore => recover(wallet_path)?,
        Interactive => {
            if path_override {
                LocalStore::from_file(wallet_path, pwd)?
            } else {
                interactive(&data_dir)?
            }
        }
        _ => LocalStore::from_file(wallet_path, pwd)?,
    };

    // connect to rusk
    let rusk = if cfg.ipc_method == "uds" {
        rusk_uds(cfg.socket_path).await
    } else {
        let rusk_addr = format!("http://{}:{}", cfg.rusk_addr, cfg.rusk_port);
        let prov_addr = {
            let host = cfg.prover_addr.as_ref().unwrap_or(&cfg.rusk_addr);
            let port = cfg.prover_port.as_ref().unwrap_or(&cfg.rusk_port);
            format!("http://{}:{}", host, port)
        };
        rusk_tcp(rusk_addr, prov_addr).await
    };

    // create our wallet
    let wallet = match rusk {
        Ok(clients) => {
            let prover = Prover::new(
                clients.prover,
                clients.state.clone(),
                clients.network,
            );
            let state = State::new(clients.state);
            CliWallet::new(store, state, prover)
        }
        Err(_) => CliWallet::offline(store),
    };

    // run command(s)
    match cmd {
        Interactive => wallet.interactive(),
        _ => {
            // in headless mode we only print the tx hash for convenience
            if let Some(txh) = wallet.run(cmd)? {
                println!("\r{}", txh);
            }
            Ok(())
        }
    }
}

/// Create a new wallet
fn create(mut path: PathBuf, skip_recovery: bool) -> Result<LocalStore, Error> {
    // prevent user from overwriting an existing wallet file
    while path.is_file() {
        let name = prompt::request_wallet_name();
        path.set_file_name(name);
        path.set_extension("dat");
    }

    // generate mnemonic and seed
    let ms = MnemSeed::new("");
    if !skip_recovery {
        prompt::confirm_recovery_phrase(ms.phrase);
    }

    // ask user for a password to secure the wallet
    let pwd = prompt::create_password();

    // create the store and attempt to write it to disk
    let store = LocalStore::new(path.clone(), ms.seed)?;
    store.save(pwd)?;

    // inform the user and return
    println!(
        "> Your new wallet was created: {}",
        path.as_os_str().to_str().unwrap()
    );
    Ok(store)
}

/// Recover access to a lost wallet file
fn recover(mut path: PathBuf) -> Result<LocalStore, Error> {
    // prevent user from overwriting an existing wallet file
    while path.is_file() {
        let name = prompt::request_wallet_name();
        path.set_file_name(name);
        path.set_extension("dat");
    }

    // ask user for 12-word recovery phrase
    let phrase = prompt::request_recovery_phrase();

    // generate wallet seed
    let ms = MnemSeed::from_phrase(&phrase, "")?;

    // ask user for a password to secure the wallet
    let pwd = prompt::create_password();

    // create the store and attempt to write it to disk
    let store = LocalStore::new(path.clone(), ms.seed)?;
    store.save(pwd)?;

    // inform the user and return
    println!(
        "> Your wallet was restored succesfully: {}",
        path.as_os_str().to_str().unwrap()
    );
    Ok(store)
}

/// Loads the store interactively
fn interactive(dir: &PathBuf) -> Result<LocalStore, Error> {
    // find existing wallets
    let default = LocalStore::default_wallet();
    let wallets = LocalStore::find_wallets(dir)?;

    // let the user choose one
    if !wallets.is_empty() {
        let wallet = prompt::select_wallet(dir, wallets);
        if let Some(w) = wallet {
            let pwd =
                prompt::request_auth("Please enter your wallet's password");
            let store = LocalStore::from_file(w, pwd)?;
            Ok(store)
        } else {
            let action = prompt::welcome();
            match action {
                1 => Ok(create(default, false)?),
                2 => Ok(recover(default)?),
                _ => Err(Error::UserExit),
            }
        }
    } else {
        println!(
            "No wallet files found at {}",
            &dir.as_os_str().to_str().unwrap_or("?")
        );
        let action = prompt::welcome();
        match action {
            1 => Ok(create(default, false)?),
            2 => Ok(recover(default)?),
            _ => Err(Error::UserExit),
        }
    }
}
