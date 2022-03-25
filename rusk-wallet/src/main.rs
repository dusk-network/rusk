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
use std::path::{Path, PathBuf};
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use lib::clients::{Prover, State};
use lib::config::Config;
use lib::crypto::MnemSeed;
use lib::prompt;
use lib::store::LocalStore;
use lib::wallet::CliWallet;

use rusk_proto::network_client::NetworkClient;
use rusk_proto::prover_client::ProverClient;
use rusk_proto::state_client::StateClient;

/// The CLI Wallet
#[derive(Parser)]
#[clap(version)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(about = "A user-friendly, reliable command line interface to the Dusk wallet!", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub(crate) struct WalletArgs {
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

    /// IPC method for communication with rusk [uds, tcp_ip]
    #[clap(short = 'i', long)]
    ipc_method: Option<String>,

    /// Rusk address: socket path or fully quallified URL
    #[clap(short = 'r', long)]
    rusk_addr: Option<String>,

    /// Prover service address
    #[clap(short = 'p', long)]
    prover_addr: Option<String>,

    /// Skip wallet recovery phrase (useful for headless wallet creation)
    #[clap(long)]
    skip_recovery: Option<bool>,

    /// Command
    #[clap(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Clone, Subcommand)]
enum CliCommand {
    /// Create a new wallet
    Create,

    /// Restore a lost wallet
    Restore,

    /// Check your current balance
    Balance {
        /// Key index
        #[clap(short, long, default_value_t = 0)]
        key: u64,
    },

    /// Retrieve public spend key
    Address {
        /// Key index
        #[clap(short, long, default_value_t = 0)]
        key: u64,
    },

    /// Send Dusk through the network
    Transfer {
        /// Key index from which to send Dusk
        #[clap(short, long, default_value_t = 0)]
        key: u64,

        /// Receiver address
        #[clap(short, long)]
        rcvr: String,

        /// Amount of Dusk to send (in µDusk)
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: Option<u64>,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Start staking Dusk
    Stake {
        /// Key index from which to stake Dusk
        #[clap(short, long, default_value_t = 0)]
        key: u64,

        /// Staking key to sign this stake
        #[clap(short, long, default_value_t = 0)]
        stake_key: u64,

        /// Amount of Dusk to stake (in µDusk)
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: Option<u64>,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Check your stake
    StakeInfo {
        /// Staking key used to sign the stake
        #[clap(short, long, default_value_t = 0)]
        key: u64,
    },

    /// Withdraw a key's stake
    WithdrawStake {
        /// Key index from which your Dusk was staked
        #[clap(short, long, default_value_t = 0)]
        key: u64,

        /// Staking key index used for this stake
        #[clap(short, long, default_value_t = 0)]
        stake_key: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: Option<u64>,

        /// Max price you're willing to pay for gas used (in µDusk)
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },

    /// Export BLS provisioner key pair
    Export {
        /// Key index from which your Dusk was staked
        #[clap(short, long, default_value_t = 0)]
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
async fn rusk_tcp(rusk_addr: &str, prov_addr: &str) -> Result<Rusk, Error> {
    Ok(Rusk {
        network: NetworkClient::connect(rusk_addr.to_string())
            .await
            .map_err(Error::RuskConn)?,
        state: StateClient::connect(rusk_addr.to_string())
            .await
            .map_err(Error::RuskConn)?,
        prover: ProverClient::connect(prov_addr.to_string())
            .await
            .map_err(Error::ProverConn)?,
    })
}

/// Connect to rusk via UDS (Unix domain sockets)
async fn rusk_uds(socket_path: &str) -> Result<Rusk, Error> {
    let socket_path = socket_path.to_string();
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

    // parse user args
    let args = WalletArgs::parse();
    let cmd = args.command.clone();

    // data directory needs to be clear from the start
    let data_dir = args
        .data_dir
        .as_ref()
        .cloned()
        .unwrap_or_else(LocalStore::default_data_dir);

    // create directories
    LocalStore::create_dir(&data_dir)?;

    // load configuration (or use default)
    let mut cfg = Config::load(data_dir)?;

    // merge static config with parsed args
    cfg.merge(args);

    // get command or default to interactive mode
    let cmd = cmd.unwrap_or(CliCommand::Interactive);

    // request auth for wallet (if required)
    let pwd = if cmd.uses_wallet() || cfg.wallet.file.is_some() {
        prompt::request_auth("Please enter wallet password")
    } else {
        blake3::hash("".as_bytes())
    };

    // prepare wallet path
    let mut path_override = false;
    let wallet_path = match cfg.wallet.file {
        Some(ref p) => {
            path_override = true;
            p.with_extension("dat")
        }
        None => {
            let mut pb = PathBuf::new();
            pb.push(&cfg.wallet.data_dir);
            pb.push(&cfg.wallet.name);
            pb.set_extension("dat");
            pb
        }
    };

    // creating and restoring are on their own
    match cmd {
        Create => {
            create(&wallet_path, cfg.wallet.skip_recovery)?;
            exit();
        }
        Restore => {
            recover(&wallet_path)?;
            exit();
        }
        _ => (),
    }

    // load our store
    let store = match cmd {
        Interactive => {
            if path_override {
                LocalStore::from_file(&wallet_path, pwd)?
            } else {
                open_interactive(&cfg)?
            }
        }
        _ => LocalStore::from_file(&wallet_path, pwd)?,
    };

    // connect to rusk
    let rusk = if cfg.rusk.ipc_method == "uds" {
        rusk_uds(&cfg.rusk.rusk_addr).await
    } else {
        rusk_tcp(&cfg.rusk.rusk_addr, &cfg.rusk.prover_addr).await
    };

    // create our wallet
    let wallet = match rusk {
        Ok(clients) => {
            let prover = Prover::new(
                clients.prover,
                clients.state.clone(),
                clients.network,
            );
            let state = State::new(
                clients.state,
                cfg.chain.gql_url.clone(),
                cfg.wallet.data_dir.as_path(),
            )?;
            CliWallet::new(cfg, store, state, prover)
        }
        Err(err) => {
            println!("{}", err);
            CliWallet::offline(cfg, store)
        }
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
fn create(path: &Path, skip_recovery: bool) -> Result<LocalStore, Error> {
    // generate mnemonic and seed
    let ms = MnemSeed::new("");
    if !skip_recovery {
        prompt::confirm_recovery_phrase(ms.phrase);
    }

    // ask user for a password to secure the wallet
    let pwd = prompt::create_password();

    // create the store and attempt to write it to disk
    let store = LocalStore::new(path, ms.seed)?;
    store.save(pwd)?;

    // inform the user and return
    println!("> Your new wallet was created: {}", path.display());
    Ok(store)
}

/// Recover access to a lost wallet file
fn recover(path: &Path) -> Result<LocalStore, Error> {
    // ask user for 12-word recovery phrase
    let phrase = prompt::request_recovery_phrase();

    // generate wallet seed
    let ms = MnemSeed::from_phrase(&phrase, "")?;

    // ask user for a password to secure the wallet
    let pwd = prompt::create_password();

    // create the store and attempt to write it to disk
    let store = LocalStore::new(path, ms.seed)?;
    store.save(pwd)?;

    // inform the user and return
    println!("> Your wallet was restored succesfully: {}", path.display());
    Ok(store)
}

/// Loads the store interactively
fn open_interactive(cfg: &Config) -> Result<LocalStore, Error> {
    // find existing wallets
    let wallets = LocalStore::wallets_in(&cfg.wallet.data_dir)?;
    if !wallets.is_empty() {
        // let the user choose one
        let wallet = prompt::choose_wallet(&wallets);
        if let Some(p) = wallet {
            let pwd =
                prompt::request_auth("Please enter your wallet's password");
            let store = LocalStore::from_file(&p, pwd)?;
            Ok(store)
        } else {
            Ok(first_run(cfg)?)
        }
    } else {
        println!("No wallet files found at {}", cfg.wallet.data_dir.display());
        Ok(first_run(cfg)?)
    }
}

/// Welcome the user when no wallets are found
fn first_run(cfg: &Config) -> Result<LocalStore, Error> {
    // greet the user and ask for action
    let action = prompt::welcome();
    if action == 0 {
        exit();
    }

    // let the user pick a name
    let name = prompt::request_wallet_name(&cfg.wallet.data_dir);
    let mut p = cfg.wallet.data_dir.clone();
    p.push(name);
    p.set_extension("dat");

    // create the store
    match action {
        1 => Ok(create(&p, false)?),
        2 => Ok(recover(&p)?),
        _ => panic!("unrecongnized option"),
    }
}

/// Terminates the program immediately with no errors
fn exit() {
    std::process::exit(0);
}
