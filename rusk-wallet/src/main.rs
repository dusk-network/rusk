// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(crate) mod rusk_proto {
    tonic::include_proto!("rusk");
}

mod lib;
pub use lib::error::Error;

use clap::{AppSettings, Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
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
/// Default data directory name
pub(crate) const DATA_DIR: &str = ".dusk";
/// Default UDS path that Rusk GRPC-server will connect to
pub(crate) const RUSK_SOCKET: &str = "/tmp/rusk_listener";

/// The CLI Wallet
#[derive(Parser)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(version = "0.2.1")]
#[clap(about = "Easily manage your Dusk", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
//#[clap(global_setting(AppSettings::SubcommandRequiredElseHelp))]
pub(crate) struct WalletCfg {
    /// Directory to store user data
    #[clap(short, long, default_value_t = WalletCfg::default_data_dir())]
    data_dir: String,

    /// Name for your wallet
    #[clap(short = 'n', long, value_name = "NAME", default_value_t = WalletCfg::default_wallet_name())]
    wallet_name: String,

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

    /// IPC method for communication with rusk [uds, tcp_ip]
    #[clap(short = 'i', long, default_value_t = WalletCfg::default_ipc_method())]
    ipc_method: String,

    /// Path for setting up the unix domain socket
    #[clap(short = 's', long, default_value_t = RUSK_SOCKET.to_string())]
    socket_path: String,

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

        /// Amount of Dusk to send
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: u64,

        /// Max price you're willing to pay for gas used
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

        /// Amount of Dusk to stake
        #[clap(short, long)]
        amt: u64,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long)]
        gas_limit: u64,

        /// Max price you're willing to pay for gas used
        #[clap(short = 'p', long)]
        gas_price: Option<u64>,
    },
    /*
        /// Extend stake for a particular key
        ExtendStake {
            /// Key index from which your Dusk was staked
            #[clap(short, long)]
            key: u64,

            /// Staking key index used for this stake
            #[clap(short, long)]
            stake_key: u64,

            /// Max amt of gas for this transaction
            #[clap(short = 'l', long)]
            gas_limit: u64,

            /// Max price you're willing to pay for gas used
            #[clap(short = 'p', long)]
            gas_price: Option<u64>,
        },
    */
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

        /// Max price you're willing to pay for gas used
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

impl WalletCfg {
    /// Default data directory is in user's home dir
    fn default_data_dir() -> String {
        let home = dirs::home_dir().expect("OS not supported");
        let path = Path::new(home.as_os_str()).join(DATA_DIR);
        String::from(path.to_str().unwrap())
    }

    /// Default wallet name is essentially current username
    fn default_wallet_name() -> String {
        // get default user as default wallet name (remove whitespace)
        let mut user: String = whoami::username();
        user.retain(|c| !c.is_whitespace());
        user.push_str(".dat");
        user
    }

    /// Default transport method for communication with Rusk
    fn default_ipc_method() -> String {
        "uds".to_string()
    }

    /// Checks consistency of loaded configuration
    fn sanity_check(&self) -> Result<(), Error> {
        // TODO!
        Ok(())
    }
}

/// Client connections to rusk Services
struct Rusk {
    network: NetworkClient<Channel>,
    state: StateClient<Channel>,
    prover: ProverClient<Channel>,
}

/// Connect to rusk services via TCP
async fn rusk_tcp(addr: &str, port: &str) -> Result<Rusk, Error> {
    let rusk_addr = format!("http://{}:{}", addr, port);
    Ok(Rusk {
        network: NetworkClient::connect(rusk_addr.clone()).await?,
        state: StateClient::connect(rusk_addr.clone()).await?,
        prover: ProverClient::connect(rusk_addr).await?,
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
        .await
        .expect("Error generating a UDS Channel, try TCP");

    Ok(Rusk {
        network: NetworkClient::new(channel.clone()),
        state: StateClient::new(channel.clone()),
        prover: ProverClient::new(channel),
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    use CliCommand::*;

    // parse cli arguments
    let cfg: WalletCfg = WalletCfg::parse();
    cfg.sanity_check()?;

    // make sure directory exists
    fs::create_dir_all(&cfg.data_dir)?;

    // get command or default to interactive mode
    let cmd = cfg.command.unwrap_or(CliCommand::Interactive);

    // prepare wallet path
    let wallet_path = if let Some(p) = cfg.wallet_file {
        p.with_extension("dat")
    } else {
        let mut pb = PathBuf::new();
        pb.push(&cfg.data_dir);
        pb.push(&cfg.wallet_name);
        pb.set_extension("dat");
        pb
    };

    // request auth for wallet (if required)
    let pwd = if cmd.uses_wallet() {
        prompt::request_auth("Please enter your wallet's password")
    } else {
        blake3::hash("".as_bytes())
    };

    // start our local store
    let store = match cmd {
        Create => create(wallet_path)?,
        Restore => recover(wallet_path)?,
        Interactive => interactive(wallet_path)?,
        _ => LocalStore::from_file(wallet_path, pwd)?,
    };

    // create the wallet
    let rusk = if cfg.ipc_method == "uds" {
        rusk_uds(cfg.socket_path).await
    } else {
        rusk_tcp(&cfg.rusk_addr, &cfg.rusk_port).await
    };

    let wallet = match rusk {
        Ok(clients) => {
            let prover = Prover::new(clients.prover, clients.network);
            let state = State::new(clients.state);
            CliWallet::new(store, state, prover)
        }
        Err(_) => CliWallet::offline(store),
    };

    // run command(s)
    match cmd {
        Interactive => wallet.interactive(),
        _ => wallet.run(cmd),
    }
}

/// Create a new wallet
fn create(mut path: PathBuf) -> Result<LocalStore, Error> {
    // prevent user from overwriting an existing wallet file
    while path.is_file() {
        let name = prompt::request_wallet_name();
        path.set_file_name(name);
        path.set_extension("dat");
    }

    // generate mnemonic and seed
    let ms = MnemSeed::new("");
    prompt::confirm_recovery_phrase(ms.phrase);

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
fn interactive(path: PathBuf) -> Result<LocalStore, Error> {
    // find existing wallets
    let dir = WalletCfg::default_data_dir();
    let wallets = find_wallets(&dir)?;

    // let the user choose one
    if !wallets.is_empty() {
        let wallet = prompt::select_wallet(&dir, wallets);
        if let Some(w) = wallet {
            let pwd =
                prompt::request_auth("Please enter your wallet's password");
            let store = LocalStore::from_file(w, pwd)?;
            Ok(store)
        } else {
            let action = prompt::welcome();
            match action {
                1 => Ok(create(path)?),
                2 => Ok(recover(path)?),
                _ => Err(Error::UserExit),
            }
        }
    }
    // nothing found
    else {
        println!("No wallet files found at {}", &dir);
        let action = prompt::welcome();
        match action {
            1 => Ok(create(path)?),
            2 => Ok(recover(path)?),
            _ => Err(Error::UserExit),
        }
    }
}

/// Scan data directory and return a list of filenames
fn find_wallets(dir: &str) -> Result<Vec<String>, Error> {
    // scan for wallets
    let dir = fs::read_dir(dir)?;
    let names = dir
        .map(|entry| {
            entry
                .ok()
                .and_then(|e| {
                    e.path()
                        .file_name()
                        .and_then(|name| name.to_str().map(String::from))
                })
                .unwrap()
        })
        .collect::<Vec<String>>();
    Ok(names)
}
