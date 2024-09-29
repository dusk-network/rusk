// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod history;

use clap::Subcommand;
use execution_core::transfer::data::{ContractCall, TransactionData};
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

    /// Check your current Phoenix balance
    PhoenixBalance {
        /// Address
        #[clap(short, long)]
        addr: Option<Address>,

        /// Check maximum spendable balance
        #[clap(long)]
        spendable: bool,
    },

    /// Check your current Moonlight balance
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

    // Phoenix transaction commands
    /// Show address transaction history
    PhoenixHistory {
        /// Address for which you want to see the history
        #[clap(short, long)]
        addr: Option<Address>,
    },

    /// Send DUSK privately through the network using Phoenix
    PhoenixTransfer {
        /// Phoenix address from which to send DUSK [default: first address]
        #[clap(short, long)]
        sndr: Option<Address>,

        /// Phoenix receiver address
        #[clap(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Stake DUSK through Phoenix
    PhoenixStake {
        /// Phoenix address from which to stake DUSK [default: first address]
        #[clap(short = 's', long)]
        addr: Option<Address>,

        /// Amount of DUSK to stake
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Unstake using Phoenix
    PhoenixUnstake {
        /// Phoenix address from which to make the unstake request [default:
        /// first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Withdraw accumulated rewards for a stake key using Phoenix
    PhoenixWithdraw {
        /// Phoenix address from which to make the withdraw request [default:
        /// first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Deploy a contract using Phoenix
    PhoenixContractDeploy {
        /// Phoenix address from which to deploy the contract [default: first]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Path to the WASM contract code
        #[clap(short, long)]
        code: PathBuf,

        /// Arguments for init function
        #[clap(short, long)]
        init_args: Vec<u8>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Call a contract using Phoenix
    PhoenixContractCall {
        /// Phoenix address from which to call the contract [default: first]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Contract id of the contract to call
        #[clap(short, long)]
        contract_id: Vec<u8>,

        /// Function name to call
        #[clap(short, long)]
        fn_name: String,

        /// Function arguments for this call
        #[clap(short, long)]
        fn_args: Vec<u8>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Attach a memo to a transaction
    PhoenixMemo {
        /// Phoenix address from which to call the contract [default: first]
        #[clap(short, long)]
        addr: Option<Address>,

        /// memo to attach to the transaction
        #[clap(short, long)]
        memo: String,

        /// Max amount of gas for this transaction
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

    /// Moonlight transaction commands
    /// Send DUSK publicly through the network using Moonlight
    MoonlightTransfer {
        /// Moonlight Address from which to send DUSK [default: first address]
        #[clap(short, long)]
        sndr: Option<Address>,

        /// Moonlight receiver address
        #[clap(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Stake DUSK using Moonlight
    MoonlightStake {
        /// Moonlight address from which to stake DUSK [default: first address]
        #[clap(short = 's', long)]
        addr: Option<Address>,

        /// Amount of DUSK to stake
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Unstake using Moonlight
    MoonlightUnstake {
        /// Moonlight address from which to make the unstake request [default:
        /// first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Withdraw accumulated rewards for a stake key using Moonlight
    MoonlightWithdraw {
        /// Moonlight address from which to make the withdraw request [default:
        /// first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Amount of DUSK to withdraw
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Deploy a contract using Moonlight
    MoonlightContractDeploy {
        /// Moonlight address from which to deploy the contract [default:
        /// first]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Path to the WASM contract code
        #[clap(short, long)]
        code: PathBuf,

        /// Arguments for init function
        #[clap(short, long)]
        init_args: Vec<u8>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Call a contract using Moonlight
    MoonlightContractCall {
        /// ContractId to call
        #[clap(short, long)]
        addr: Option<Address>,

        /// contract id of the contract to call
        #[clap(short, long)]
        contract_id: Vec<u8>,

        /// Function name to call
        #[clap(short, long)]
        fn_name: String,

        /// Function arguments for this call
        #[clap(short, long)]
        fn_args: Vec<u8>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    MoonlightMemo {
        /// Moonlight address from which to send DUSK [default: first address]
        #[clap(short, long)]
        addr: Option<Address>,

        /// Memo is additonal info attached to transaction
        #[clap(short, long)]
        memo: String,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    // Conversion commands
    /// Convert Phoenix DUSK to Moonlight for the same owned address
    PhoenixToMoonlight {
        /// Moonlight or Phoenix address from which to convert DUSK to
        #[clap(short, long)]
        addr: Option<Address>,

        /// Amount of DUSK to transfer to your Moonlight account
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Convert Moonlight DUSK to Phoenix for the same owned address
    MoonlightToPhoenix {
        /// Moonlight or Phoenix Address from which to convert DUSK to
        #[clap(short, long)]
        addr: Option<Address>,

        /// Amount of DUSK to transfer to your phoenix account
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t= DEFAULT_STAKE_GAS_LIMIT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t= DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Export BLS provisioner key-pair
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
                    wallet.get_moonlight_balance(addr).await?,
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
            Command::PhoenixToMoonlight {
                addr,
                gas_limit,
                gas_price,
                amt,
            } => {
                wallet.sync().await?;
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx = wallet.phoenix_to_moonlight(addr, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightToPhoenix {
                addr,
                amt,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let tx = wallet.moonlight_to_phoenix(addr, amt, gas).await?;
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

                let tx = wallet.moonlight_stake(addr, amt, gas).await?;
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
            Command::MoonlightWithdraw {
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

                let tx =
                    wallet.moonlight_stake_withdraw(addr, amt, gas).await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::PhoenixContractCall {
                addr,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let contract_id: [u8; 32] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name, &fn_args)
                    .map_err(|_| Error::Rkyv)?;

                let tx = wallet
                    .phoenix_execute(addr, Dusk::from(0), gas, call.into())
                    .await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightContractCall {
                addr,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let contract_id: [u8; 32] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name, &fn_args)
                    .map_err(|_| Error::Rkyv)?;

                let tx = wallet
                    .moonlight_execute(
                        addr,
                        None,
                        Dusk::from(0),
                        Dusk::from(0),
                        gas,
                        call.into(),
                    )
                    .await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Self::PhoenixContractDeploy {
                addr,
                code,
                init_args,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }

                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let tx =
                    wallet.phoenix_deploy(addr, code, init_args, gas).await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Self::MoonlightContractDeploy {
                addr,
                code,
                init_args,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }

                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let tx =
                    wallet.moonlight_deploy(addr, code, init_args, gas).await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Self::MoonlightMemo {
                addr,
                memo,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let memo = memo.as_bytes().to_vec();

                let tx = wallet
                    .moonlight_execute(
                        addr,
                        None,
                        Dusk::from(0),
                        Dusk::from(0),
                        gas,
                        Some(TransactionData::Memo(memo)),
                    )
                    .await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Self::PhoenixMemo {
                addr,
                memo,
                gas_limit,
                gas_price,
            } => {
                let addr = match addr {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let memo = memo.as_bytes().to_vec();

                let tx = wallet
                    .phoenix_execute(
                        addr,
                        Dusk::from(0),
                        gas,
                        TransactionData::Memo(memo),
                    )
                    .await?;

                Ok(RunResult::Tx(tx.hash()))
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
                let total = Dusk::from(balance.value);
                let spendable = Dusk::from(balance.spendable);
                write!(
                    f,
                    "> Total Phoenix balance is: {total} DUSK\n\
                     > Maximum spendable per TX is: {spendable} DUSK",
                )
            }
            MoonlightBalance(balance) => {
                write!(f, "> Total Moonlight balance is: {balance} DUSK")
            }
            Address(addr) => {
                write!(f, "> {addr}")
            }
            Addresses(addrs) => {
                let str_addrs = addrs
                    .iter()
                    .map(|a| format!("{a}"))
                    .collect::<Vec<String>>()
                    .join("\n> ");
                write!(f, "> {}", str_addrs)
            }
            Tx(hash) => {
                let hash = hex::encode(hash.to_bytes());
                write!(f, "> Transaction sent: {hash}",)
            }
            StakeInfo(data, _) => {
                match data.amount {
                    Some(amt) => {
                        let amount = Dusk::from(amt.value);
                        let locked = Dusk::from(amt.locked);
                        let eligibility = amt.eligibility;
                        let epoch = amt.eligibility / EPOCH;

                        writeln!(f, "> Eligible stake amount: {amount} DUSK")?;
                        if locked > 0 {
                            writeln!(f, "> Locked amount: {locked} DUSK")?;
                        };
                        writeln!(f, "> Stake eligibility from block #{eligibility} (Epoch {epoch})")
                    }
                    None => writeln!(f, "> No active stake found for this key"),
                }?;
                let reward = Dusk::from(data.reward);
                write!(f, "> Accumulated reward is: {reward} DUSK")
            }
            ExportedKeys(pk, kp) => {
                let pk = pk.display();
                let kp = kp.display();
                write!(
                    f,
                    "> Public key exported to: {pk}\n\
                     > Key pair exported to: {kp}",
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
