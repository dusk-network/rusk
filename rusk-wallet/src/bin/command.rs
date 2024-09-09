// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod history;

use clap::Subcommand;
use std::{fmt, path::PathBuf};

use crate::io::prompt;
use crate::settings::Settings;
use crate::{WalletFile, WalletPath};

use execution_core::{stake::StakeData, BlsScalar};
use rusk_wallet::{
    currency::{Dusk, Lux},
    gas::{Gas, DEFAULT_LIMIT, DEFAULT_PRICE},
    Address, Error, Wallet, EPOCH, MAX_ADDRESSES,
};
use wallet_core::BalanceInfo;

pub use history::TransactionHistory;

/// The default stake gas limit
pub const DEFAULT_STAKE_GAS_LIMIT: u64 = 2_900_000_000;

/// Commands that can be run against the Dusk wallet
#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub(crate) enum Command {
    /// Create a new wallet
    Create {
        /// Skip wallet recovery phrase (useful for headless wallet creation)
        #[clap(long, action)]
        skip_recovery: bool,

        /// Save recovery phrase to file (useful for headless wallet creation)
        #[clap(long)]
        seed_file: Option<PathBuf>,
    },

    /// Restore a lost wallet
    Restore {
        /// Set the wallet .dat file to restore from
        #[clap(short, long)]
        file: Option<WalletPath>,
    },

    /// Check your current phoenix balance
    PhoenixBalance {
        /// Address
        #[clap(short, long)]
        addr: Option<Address>,

        /// Check maximum spendable balance
        #[clap(long)]
        spendable: bool,
    },

    /// Check your current moonlight balance
    MoonlightBalance {
        /// Address
        #[clap(short, long)]
        addr: Option<Address>,
    },

    /// List your existing addresses and generate new ones
    Addresses {
        /// Create new address
        #[clap(short, long, action)]
        new: bool,
    },

    /// Show address transaction history
    PhoenixHistory {
        /// Address for which you want to see the history
        #[clap(short, long)]
        addr: Option<Address>,
    },

    /// Send DUSK privately through the network using Phoenix
    PhoenixTransfer {
        /// Phoenix Address from which to send DUSK [default: first address]
        #[clap(short, long)]
        sndr: Option<Address>,

        /// Phoenix Receiver address
        #[clap(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[clap(short, long)]
        amt: Dusk,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Send DUSK through the network using moonlight
    MoonlightTransfer {
        /// Bls Address from which to send DUSK [default: first address]
        #[clap(short, long)]
        sndr: Option<Address>,

        /// Bls Receiver address
        #[clap(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[clap(short, long)]
        amt: Dusk,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Start staking DUSK through phoenix
    PhoenixStake {
        /// Phoenix Address from which to stake DUSK [default: first address]
        #[clap(short = 's', long)]
        addr: Option<Address>,

        /// Amount of DUSK to stake
        #[clap(short, long)]
        amt: Dusk,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Start staking DUSK through moonlight
    MoonlightStake {
        /// Bls Address from which to stake DUSK [default: first address]
        #[clap(short = 's', long)]
        addr: Option<Address>,

        /// Amount of DUSK to stake
        #[clap(short, long)]
        amt: Dusk,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Check your stake information
    StakeInfo {
        /// Address used to stake [default: first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Check accumulated reward
        #[clap(long, action)]
        reward: bool,
    },

    /// Phoeinx Unstake a key's stake
    PhoenixUnstake {
        /// Phoenix Address from which your DUSK was staked [default: first
        /// address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Moonlight Unstake a key's stake
    MoonlightUnstake {
        /// Bls Address from which your DUSK was staked [default: first
        /// address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Phoenix Withdraw accumulated reward for a stake key
    PhoenixWithdraw {
        /// Phoenix Address from which your DUSK was staked [default: first
        /// address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amt of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Export BLS provisioner key pair
    Export {
        /// Address for which you want the exported keys [default: first
        /// address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Output directory for the exported keys
        #[clap(short, long)]
        dir: PathBuf,

        /// Name of the files exported [default: staking-address]
        #[clap(short, long)]
        name: Option<String>,
    },

    /// Show current settings
    Settings,
}

impl Command {
    /// Runs the command with the provided wallet
    pub async fn run(
        self,
        wallet: &mut Wallet<WalletFile>,
        settings: &Settings,
    ) -> anyhow::Result<RunResult> {
        match self {
            Command::PhoenixBalance { addr, spendable } => {
                let sync_result = wallet.sync().await;
                if let Err(e) = sync_result {
                    // Sync error should be reported only if wallet is online
                    if wallet.is_online().await {
                        tracing::error!("Unable to update the balance {e:?}")
                    }
                }

                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let balance = wallet.get_phoenix_balance(addr).await?;
                Ok(RunResult::PhoenixBalance(balance, spendable))
            }
            Command::MoonlightBalance { addr } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                Ok(RunResult::MoonlightBalance(
                    wallet.get_moonlight_balance(addr)?,
                ))
            }
            Command::Addresses { new } => {
                if new {
                    if wallet.addresses().len() >= MAX_ADDRESSES {
                        println!(
                            "Cannot create more addresses, this wallet only supports up to {MAX_ADDRESSES} addresses. You have {} addresses already.", wallet.addresses().len()
                        );
                        std::process::exit(0);
                    }

                    let addr = wallet.new_address().clone();
                    wallet.save()?;
                    Ok(RunResult::Address(Box::new(addr)))
                } else {
                    Ok(RunResult::Addresses(wallet.addresses().clone()))
                }
            }
            Command::PhoenixTransfer {
                sndr,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;
                let sender = match sndr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx =
                    wallet.phoenix_transfer(sender, &rcvr, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightTransfer {
                sndr,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            } => {
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let sender = match sndr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let tx =
                    wallet.moonlight_transfer(sender, &rcvr, amt, gas).await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::PhoenixStake {
                addr,
                amt,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let tx = wallet.phoenix_stake(addr, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightStake {
                addr,
                amt,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx = wallet.moonlight_stake(addr, amt, gas)?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::StakeInfo { addr, reward } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let si = wallet
                    .stake_info(addr.index()?)
                    .await?
                    .ok_or(Error::NotStaked)?;

                Ok(RunResult::StakeInfo(si, reward))
            }
            Command::PhoenixUnstake {
                addr,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx = wallet.phoenix_unstake(addr, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightUnstake {
                addr,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = wallet.moonlight_unstake(addr, gas).await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::PhoenixWithdraw {
                addr,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx = wallet.phoenix_stake_withdraw(addr, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::Export { addr, dir, name } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let pwd = prompt::request_auth(
                    "Provide a password for your provisioner keys",
                    &settings.password,
                    wallet.get_file_version()?,
                )?;

                let (pub_key, key_pair) =
                    wallet.export_provisioner_keys(addr, &dir, name, &pwd)?;

                Ok(RunResult::ExportedKeys(pub_key, key_pair))
            }
            Command::PhoenixHistory { addr } => {
                wallet.sync().await?;
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let notes = wallet.get_all_notes(addr).await?;

                let transactions =
                    history::transaction_from_notes(settings, notes).await?;

                Ok(RunResult::PhoenixHistory(transactions))
            }
            Command::Create { .. } => Ok(RunResult::Create()),
            Command::Restore { .. } => Ok(RunResult::Restore()),
            Command::Settings => Ok(RunResult::Settings()),
        }
    }
}

/// Possible results of running a command in interactive mode
pub enum RunResult {
    Tx(BlsScalar),
    PhoenixBalance(BalanceInfo, bool),
    MoonlightBalance(Dusk),
    StakeInfo(StakeData, bool),
    Address(Box<Address>),
    Addresses(Vec<Address>),
    ExportedKeys(PathBuf, PathBuf),
    Create(),
    Restore(),
    Settings(),
    PhoenixHistory(Vec<TransactionHistory>),
}

impl fmt::Display for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RunResult::*;
        match self {
            PhoenixBalance(balance, _) => {
                write!(
                    f,
                    "> Total Phoenix balance is: {} DUSK\n> Maximum spendable per TX is: {} DUSK",
                    Dusk::from(balance.value),
                    Dusk::from(balance.spendable)
                )
            }
            MoonlightBalance(balance) => {
                write!(f, "> Total Moonlight balance is: {} DUSK", balance)
            }
            Address(addr) => {
                write!(f, "> {}", addr)
            }
            Addresses(addrs) => {
                let str_addrs = addrs
                    .iter()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join("\n>");
                write!(f, "> {}", str_addrs)
            }
            Tx(hash) => {
                let hash = hex::encode(hash.to_bytes());
                write!(f, "> Transaction sent: {hash}",)
            }
            StakeInfo(data, _) => {
                let stake_str = match data.amount {
                    Some(amt) => format!(
                        "Current stake amount is: {} DUSK\n> Stake eligibility from block #{} (Epoch {})",
                        Dusk::from(amt.value),
                        amt.eligibility,
                        amt.eligibility / EPOCH
                    ),
                    None => "No active stake found for this key".to_string(),
                };
                write!(
                    f,
                    "> {}\n> Accumulated reward is: {} DUSK",
                    stake_str,
                    Dusk::from(data.reward)
                )
            }
            ExportedKeys(pk, kp) => {
                write!(
                    f,
                    "> Public key exported to: {}\n> Key pair exported to: {}",
                    pk.display(),
                    kp.display()
                )
            }
            PhoenixHistory(transactions) => {
                writeln!(f, "{}", TransactionHistory::header())?;
                for th in transactions {
                    writeln!(f, "{th}")?;
                }
                Ok(())
            }
            Create() | Restore() | Settings() => unreachable!(),
        }
    }
}
