// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod history;

pub use history::TransactionHistory;

use clap::Subcommand;
use execution_core::{
    stake::StakeData, transfer::data::ContractCall, BlsScalar,
    CONTRACT_ID_BYTES,
};
use rusk_wallet::{
    currency::{Dusk, Lux},
    gas::{
        Gas, DEFAULT_LIMIT_CALL, DEFAULT_LIMIT_DEPLOYMENT,
        DEFAULT_LIMIT_TRANSFER, DEFAULT_PRICE,
    },
    Address, Error, Wallet, EPOCH, MAX_ADDRESSES,
};
use wallet_core::BalanceInfo;

use crate::io::prompt;
use crate::settings::Settings;
use crate::{WalletFile, WalletPath};

use std::{fmt, path::PathBuf};

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

    /// Check your current balance
    Balance {
        /// Address
        #[clap(short, long)]
        address: Option<Address>,
        /// Check maximum spendable balance
        #[clap(long)]
        spendable: bool,
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
        /// Address index for which you want to see the history
        #[clap(short, long)]
        addr_idx: Option<u8>,
    },

    /// Send DUSK through the network
    Transfer {
        /// Address from which to send DUSK [default: first address]
        #[clap(short, long)]
        sender: Option<Address>,

        /// Receiver address
        #[clap(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_TRANSFER)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,

        /// Optional memo to attach to the transaction
        #[clap(long)]
        memo: Option<String>,
    },

    /// Stake DUSK
    Stake {
        /// Address from which to stake DUSK [default: first address]
        #[clap(short = 's', long)]
        address: Option<Address>,

        /// Amount of DUSK to stake
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Unstake DUSK
    Unstake {
        /// Address from which to make the unstake request [default: first
        /// address]
        #[clap(short, long)]
        address: Option<Address>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Withdraw accumulated rewards for a stake key
    Withdraw {
        /// Address from which to make the withdraw request [default:
        /// first address]
        #[clap(short, long)]
        address: Option<Address>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Deploy a contract
    ContractDeploy {
        /// Address from which to deploy the contract [default: first]
        #[clap(short, long)]
        address: Option<Address>,

        /// Path to the WASM contract code
        #[clap(short, long)]
        code: PathBuf,

        /// Arguments for init function
        #[clap(short, long)]
        init_args: Vec<u8>,

        /// Nonce used for the deploy transaction
        #[clap(short, long)]
        deploy_nonce: u64,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_DEPLOYMENT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },
    /// Check your stake information
    StakeInfo {
        /// Address used to stake [default: first address]
        #[clap(short, long)]
        addr_idx: Option<u8>,

        /// Check accumulated reward
        #[clap(long, action)]
        reward: bool,
    },

    /// Call a contract
    ContractCall {
        /// Address from which to call the contract [default: first]
        #[clap(short, long)]
        address: Option<Address>,

        /// Contract id of the contract to call
        #[clap(short, long)]
        contract_id: Vec<u8>,

        /// Function name to call
        #[clap(short = 'n', long)]
        fn_name: String,

        /// Function arguments for this call
        #[clap(short = 'f', long)]
        fn_args: Vec<u8>,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    // Conversion commands
    /// Convert Phoenix DUSK to Moonlight for the same owned address
    PhoenixToMoonlight {
        /// Moonlight or Phoenix address from which to convert DUSK to
        #[clap(short = 's', long)]
        addr_idx: Option<u8>,

        /// Amount of DUSK to transfer to your Moonlight account
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Command to calculate the contract id
    /// given the contract code and deploy nonce
    CalculateContractId {
        /// Bls Public key Address to keep as owner of the contract
        #[clap(short = 's', long)]
        addr_idx: Option<u8>,

        /// Path to the WASM contract code
        #[clap(short, long)]
        code: PathBuf,

        /// Nonce used for the deploy transaction
        #[clap(short, long)]
        deploy_nonce: u64,
    },

    /// Convert Moonlight DUSK to Phoenix for the same owned address
    MoonlightToPhoenix {
        /// Moonlight or Phoenix Address from which to convert DUSK to
        #[clap(short = 's', long)]
        addr_idx: Option<u8>,

        /// Amount of DUSK to transfer to your phoenix account
        #[clap(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[clap(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[clap(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Export BLS provisioner key-pair
    Export {
        /// Address for which you want the exported keys [default: first
        /// address]
        #[clap(short, long)]
        addr_idx: Option<u8>,

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
            Command::Balance { address, spendable } => {
                let addr = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                match addr {
                    Address::Bls { index, addr: _ } => {
                        let addr_idx = index.unwrap_or_default();
                        Ok(RunResult::MoonlightBalance(
                            wallet.get_moonlight_balance(addr_idx).await?,
                        ))
                    }
                    Address::Phoenix { index, addr: _ } => {
                        let sync_result = wallet.sync().await;
                        if let Err(e) = sync_result {
                            // Sync error should be reported only if wallet is
                            // online
                            if wallet.is_online().await {
                                tracing::error!(
                                    "Unable to update the balance {e:?}"
                                )
                            }
                        }
                        let addr_idx = index.unwrap_or_default();

                        let balance =
                            wallet.get_phoenix_balance(addr_idx).await?;
                        Ok(RunResult::PhoenixBalance(balance, spendable))
                    }
                }
            }

            Command::Addresses { new } => {
                if new {
                    if wallet.addresses().len() >= MAX_ADDRESSES {
                        println!(
                            "Cannot create more addresses, this wallet only supports up to {MAX_ADDRESSES} addresses. You have {} addresses already.", wallet.addresses().len()
                        );
                        std::process::exit(0);
                    }

                    let new_addr_idx = wallet.add_address();
                    wallet.save()?;

                    // leave this hack here until `RunResult` gets an overhaul
                    let phoenix_addr = Address::Phoenix {
                        index: Some(new_addr_idx),
                        addr: *wallet.phoenix_pk(new_addr_idx)?,
                    };
                    Ok(RunResult::Address(Box::new(phoenix_addr)))
                } else {
                    let phoenix_addresses = wallet
                        .addresses()
                        .iter()
                        .enumerate()
                        .map(|(index, (phoenix_pk, _bls_pk))| {
                            Address::Phoenix {
                                index: Some(index as u8),
                                addr: *phoenix_pk,
                            }
                        })
                        .collect();

                    Ok(RunResult::Addresses(phoenix_addresses))
                }
            }
            Command::Transfer {
                sender,
                rcvr,
                amt,
                gas_limit,
                gas_price,
                memo,
            } => {
                let sender_idx = match sender {
                    Some(addr) => {
                        let own = wallet.claim_as_address(addr)?;
                        own.same_protocol(&rcvr)?;
                        own.index()?
                    }
                    None => 0,
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let memo = memo.filter(|m| !m.trim().is_empty());
                let tx = match rcvr {
                    Address::Phoenix { index: _, addr } => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_transfer(sender_idx, &addr, memo, amt, gas)
                            .await?
                    }
                    Address::Bls { index: _, addr } => {
                        wallet
                            .moonlight_transfer(
                                sender_idx, &addr, memo, amt, gas,
                            )
                            .await?
                    }
                };

                Ok(RunResult::Tx(tx.hash()))
            }

            Command::Stake {
                address,
                amt,
                gas_limit,
                gas_price,
            } => {
                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = address.index()?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Phoenix { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_stake(addr_idx, amt, gas).await
                    }
                    Address::Bls { .. } => {
                        wallet.moonlight_stake(addr_idx, amt, gas).await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::Unstake {
                address,
                gas_limit,
                gas_price,
            } => {
                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = address.index()?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Phoenix { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_unstake(addr_idx, gas).await
                    }
                    Address::Bls { .. } => {
                        wallet.moonlight_unstake(addr_idx, gas).await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::Withdraw {
                address,
                gas_limit,
                gas_price,
            } => {
                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = address.index()?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Phoenix { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_stake_withdraw(addr_idx, gas).await
                    }
                    Address::Bls { .. } => {
                        wallet.moonlight_stake_withdraw(addr_idx, gas).await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::StakeInfo { addr_idx, reward } => {
                let addr_idx = addr_idx.unwrap_or_default();
                let stake_info = wallet
                    .stake_info(addr_idx)
                    .await?
                    .ok_or(Error::NotStaked)?;

                Ok(RunResult::StakeInfo(stake_info, reward))
            }

            Command::Export {
                addr_idx,
                dir,
                name,
            } => {
                let pwd = prompt::request_auth(
                    "Provide a password for your provisioner keys",
                    &settings.password,
                    wallet.get_file_version()?,
                )?;
                let addr_idx = addr_idx.unwrap_or_default();

                let (pub_key, key_pair) = wallet
                    .export_provisioner_keys(addr_idx, &dir, name, &pwd)?;

                Ok(RunResult::ExportedKeys(pub_key, key_pair))
            }
            Command::PhoenixHistory { addr_idx } => {
                wallet.sync().await?;
                let addr_idx = addr_idx.unwrap_or_default();
                let notes = wallet.get_all_notes(addr_idx).await?;

                let transactions =
                    history::transaction_from_notes(settings, notes).await?;

                Ok(RunResult::PhoenixHistory(transactions))
            }
            Command::PhoenixToMoonlight {
                addr_idx,
                gas_limit,
                gas_price,
                amt,
            } => {
                wallet.sync().await?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let addr_idx = addr_idx.unwrap_or_default();

                let tx =
                    wallet.phoenix_to_moonlight(addr_idx, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightToPhoenix {
                addr_idx,
                amt,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let addr_idx = addr_idx.unwrap_or_default();

                let tx =
                    wallet.moonlight_to_phoenix(addr_idx, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::ContractCall {
                address,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
            } => {
                let gas = Gas::new(gas_limit).with_price(gas_price);

                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = address.index()?;

                let contract_id: [u8; CONTRACT_ID_BYTES] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name, &fn_args)
                    .map_err(|_| Error::Rkyv)?;

                let tx = match address {
                    Address::Phoenix { .. } => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_execute(
                                addr_idx,
                                Dusk::from(0),
                                gas,
                                call.into(),
                            )
                            .await
                    }
                    Address::Bls { .. } => {
                        wallet
                            .moonlight_execute(
                                addr_idx,
                                Dusk::from(0),
                                Dusk::from(0),
                                gas,
                                call.into(),
                            )
                            .await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }

            Self::ContractDeploy {
                address,
                code,
                init_args,
                deploy_nonce,
                gas_limit,
                gas_price,
            } => {
                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }

                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = address.index()?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Phoenix { .. } => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_deploy(
                                addr_idx,
                                code,
                                init_args,
                                deploy_nonce,
                                gas,
                            )
                            .await
                    }
                    Address::Bls { .. } => {
                        wallet
                            .moonlight_deploy(
                                addr_idx,
                                code,
                                init_args,
                                deploy_nonce,
                                gas,
                            )
                            .await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Self::CalculateContractId {
                addr_idx,
                code,
                deploy_nonce,
            } => {
                let addr_idx = addr_idx.unwrap_or_default();

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }

                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let contract_id =
                    wallet.get_contract_id(addr_idx, code, deploy_nonce)?;

                Ok(RunResult::ContractId(contract_id))
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
    ContractId([u8; CONTRACT_ID_BYTES]),
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
            StakeInfo(data, _) => match data.amount {
                Some(amt) => {
                    let amount = Dusk::from(amt.value);
                    let locked = Dusk::from(amt.locked);
                    let faults = data.faults;
                    let hard_faults = data.hard_faults;
                    let eligibility = amt.eligibility;
                    let epoch = amt.eligibility / EPOCH;
                    let rewards = Dusk::from(data.reward);

                    writeln!(f, "> Eligible stake: {amount} DUSK")?;
                    writeln!(f, "> Reclaimable slashed stake: {locked} DUSK")?;
                    writeln!(f, "> Slashes: {faults}")?;
                    writeln!(f, "> Hard Slashes: {hard_faults}")?;
                    writeln!(f, "> Stake active from block #{eligibility} (Epoch {epoch})")?;
                    write!(f, "> Accumulated rewards is: {rewards} DUSK")
                }
                None => write!(f, "> No active stake found for this key"),
            },
            ContractId(bytes) => {
                write!(f, "> Contract ID: {}", hex::encode(bytes))
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
