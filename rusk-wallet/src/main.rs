// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod rusk_proto {
    tonic::include_proto!("rusk");
}

mod lib;
mod prompt;

use std::path::{Path, PathBuf};
use clap::{AppSettings, Parser, Subcommand};
use tonic::transport::Channel;
use whoami;

use rusk_proto::{GetNotesOwnedByRequest, ViewKey, state_client::StateClient, prover_client::ProverClient};

use crate::lib::store::{LocalStore, StoreError};
use crate::lib::crypto::{MnemSeed, CryptoError};
use crate::lib::clients::{Prover, State, ProverError, StateError};

/// Default Rusk IP address
pub(crate) const RUSK_ADDR: &str = "127.0.0.1";
/// Default Rusk TCP port
pub(crate) const RUSK_PORT: &str = "8585";
/// Default data directory name
pub(crate) const DATA_DIR: &str = ".dusk";

/// Errors returned by this crate
#[derive(Debug)]
pub(crate) enum WalletError {
    Store(StoreError),
    Crypto(CryptoError),
    Prover(ProverError),
    State(StateError),
}

impl From<StoreError> for WalletError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<ProverError> for WalletError {
    fn from(e: ProverError) -> Self {
        Self::Prover(e)
    }
}

impl From<StateError> for WalletError {
    fn from(e: StateError) -> Self {
        Self::State(e)
    }
}

impl From<CryptoError> for WalletError {
    fn from(e: CryptoError) -> Self {
        Self::Crypto(e)
    }
}

/// The CLI Wallet
#[derive(Parser)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(version = "1.0")]
#[clap(about = "Easily manage your Dusk", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
#[clap(global_setting(AppSettings::SubcommandRequiredElseHelp))]
pub struct WalletCfg {

    /// Directory to store user data
    #[clap(short, long, default_value_t = WalletCfg::default_data_dir())]
    data_dir: String,

    /// Name for your wallet
    #[clap(short = 'n', long, value_name = "NAME", default_value_t = WalletCfg::default_wallet_name())]
    wallet_name: String,

    /// Path to a wallet file. Overrides `data-dir` and `wallet-name`, useful when loading a wallet that's not in the default directory.
    #[clap(short = 'f', long, parse(from_os_str), value_name = "PATH")]
    wallet_file: Option<PathBuf>,

    /// Rusk address
    #[clap(short = 'a', long, default_value_t = RUSK_ADDR.to_string())]
    rusk_addr: String,

    /// Rusk port
    #[clap(short = 'p', long, default_value_t = RUSK_PORT.to_string())]
    rusk_port: String,

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
        index: u64,
    },

    /// Retrieve public spend key
    Address {
        /// Key index
        #[clap(short, long)]
        index: u64,
    },

    /// Send Dusk through the network
    Transfer {
        /// Key index from which to send Dusk
        #[clap(short, long)]
        index: u64,

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

    },

    /// Stop staking Dusk
    StopStake {
        
    },

    /// Extend stake for a particular key
    ExtendStake {
        
    },

    /// Withdraw your stake
    WithdrawStake {
        
    }
}

impl CliCommand {
    fn uses_wallet(&self) -> bool {
        match *self {
            Self::Create | Self::Restore => false,
            _ => true,
        }
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
        user.retain(|c|!c.is_whitespace());
        user.push_str(".dat");
        user
    }

    /// Checks consistency of loaded configuration
    fn sanity_check(&self) -> Result<(), WalletError> {
        // TODO!
        Ok(())
    }

}

fn main() -> Result<(), WalletError> {

    // parse cli arguments
    let cfg: WalletCfg = WalletCfg::parse();
    cfg.sanity_check()?;

    // some commands don't use an existing wallet
    // get those out of the way first
    let cmd = cfg.command.unwrap();
    if !cmd.uses_wallet() {
        use CliCommand::*;
        return match cmd {
            Create => create(),
            Restore => recover(),
            _ =>  Ok(()),
        }
    }

    // -------- wallet

    // have our wallet path ready
    let wallet_path = if let Some(p) = cfg.wallet_file {
        p
    } else {
        let mut pb = PathBuf::new();
        pb.push(cfg.data_dir);
        pb.push(cfg.wallet_name);
        pb
    };

    // request user auth for wallet
    let pwd = if cmd.uses_wallet() {
        prompt::request_auth()
    } else {
        String::from("")
    };

    // start our local store
    let store = LocalStore::from_file(wallet_path, pwd)?;

    // connect to rusk services

    
    // todo

    Ok(())
}

fn create() -> Result<(), WalletError> {

    if let Some(data) = prompt::create() {
        LocalStore::new(data.0, data.1.seed)?;
    }
    Ok(())

}

fn recover() -> Result<(), WalletError> {
    Ok(())
}






/*
    println!("{:?}", args);

    // connect to rusk
    let mut state_client = StateClient::connect("https://127.0.0.1:8585").await.expect("Failed to connect to Rusk service.");

    // load the wallet
    let store = LocalStore::from_file(String::from("/Users/abel/.dusk/abel.dat"), String::from("m"))?;
    let node = CliNodeClient::new(&state_client);
    let wallet = dusk_wallet_core::Wallet::new(store, node);

    let node_client = CliNodeClient::new(state_client);
    let vk = ViewKey{}
    let res = node_client.fetch_notes(0, );

    println!("MAIN RESPONSE = {:?}", res);
*/