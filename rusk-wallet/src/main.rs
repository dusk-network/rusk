// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub(crate) mod rusk_proto {
    tonic::include_proto!("rusk");
}

mod lib;
mod prompt;
pub use lib::errors as errors;

use std::path::{Path, PathBuf};
use clap::{AppSettings, Parser, Subcommand};
use whoami;

use dusk_wallet_core::Wallet;

use lib::errors::CliError;
use lib::store::LocalStore;
use lib::clients::{Prover, State};

use rusk_proto::network_client::NetworkClient;
use rusk_proto::state_client::StateClient;
use rusk_proto::stake_client::StakeClient;
use rusk_proto::prover_client::ProverClient;

/// Default Rusk IP address
pub(crate) const RUSK_ADDR: &str = "127.0.0.1";
/// Default Rusk TCP port
pub(crate) const RUSK_PORT: &str = "8585";
/// Default data directory name
pub(crate) const DATA_DIR: &str = ".dusk";

/// The CLI Wallet
#[derive(Parser)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(version = "1.0")]
#[clap(about = "Easily manage your Dusk", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
//#[clap(global_setting(AppSettings::SubcommandRequiredElseHelp))]
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
        
    },

    None
}

impl CliCommand {
    fn uses_wallet(&self) -> bool {
        match *self {
            Self::Create | Self::Restore | Self::None => false,
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
    fn sanity_check(&self) -> Result<(), CliError> {
        // TODO!
        Ok(())
    }

}

#[tokio::main]
async fn main() -> Result<(), CliError> {

    // parse cli arguments
    let cfg: WalletCfg = WalletCfg::parse();

    cfg.sanity_check()?;

    // some commands don't use an existing wallet
    // get those out of the way first
    let cmd = cfg.command.unwrap_or_else(|| CliCommand::None);
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
    let rusk_addr = format!("http://{}:{}", cfg.rusk_addr, cfg.rusk_port);

    let network_client = NetworkClient::connect(rusk_addr.clone()).await
        .map_err(|e| CliError::Network(e))?;
    let state_client = StateClient::connect(rusk_addr.clone()).await
        .map_err(|e| CliError::Network(e))?;
    let prover_client = ProverClient::connect(rusk_addr.clone()).await
        .map_err(|e| CliError::Network(e))?;
    let stake_client = StakeClient::connect(rusk_addr).await
        .map_err(|e| CliError::Network(e))?;

    let prover = Prover::new(prover_client, network_client);
    let state = State::new(state_client, stake_client);

    // create our wallet
    let wallet = Wallet::new(store, state, prover);

    // perform whatever action user requested
    use CliCommand::*;
    match cmd {
        // Check your current balance
        Balance { index } => {
            println!("check balance for index: {:?}", index);
            let balance = wallet.get_balance(index);
            match balance {
                Ok(b) => println!("Your balance is: {} Dusk", b),
                Err(e) => println!("An error occured: {:?}", e)
             }
        },
        // Retrieve public spend key
        Address { index } => {
            println!("public spend key for index: {:?}", index)
        },
        // Send Dusk through the network
        Transfer { index, rcvr, amt, gas_limit, gas_price } => {
            println!("create tx: {} {} {} {} {}", index, rcvr, amt, gas_limit, gas_price.unwrap_or(0));
        },
        // Start staking Dusk
        Stake {} => {
            println!("create stake");
        },
        // Stop staking Dusk
        StopStake {} => {
            println!("stop stake");
        },
        // Extend stake for a particular key
        ExtendStake {} => {
            println!("extend stake");
        },
        // Withdraw your stake
        WithdrawStake {} => {
            println!("withdraw stake");
        },
        _ => {}
    }

    Ok(())
}

fn create() -> Result<(), CliError> {

    println!("Create a wallet!");
    if let Some(data) = prompt::create() {
        LocalStore::new(data.0, data.1.seed)?;
    }
    Ok(())

}

fn recover() -> Result<(), CliError> {
    Ok(())
}
