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
    Address, Error, Profile, Wallet, EPOCH, MAX_PROFILES,
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
        #[clap(long)]
        address: Option<Address>,
        /// Check maximum spendable balance
        #[clap(long)]
        spendable: bool,
    },

    /// List your existing profiles and generate new ones
    Profiles {
        /// Create new profile
        #[clap(short, long, action)]
        new: bool,
    },

    // Shielded transaction commands
    /// Show shielded transaction history
    PhoenixHistory {
        /// Profile index for which you want to see the history
        #[clap(long)]
        profile_idx: Option<u8>,
    },

    /// Send DUSK through the network
    Transfer {
        /// Address from which to send DUSK [default: first address]
        #[clap(long)]
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
        #[clap(long)]
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

    /// Call a contract using a shielded address
    PhoenixContractCall {
        /// Profile index for the shielded address that will pay for the gas
        /// [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

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

    /// Check your stake information
    StakeInfo {
        /// Profile index for the public account address to stake from
        /// [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

        /// Check accumulated reward
        #[clap(long, action)]
        reward: bool,
    },

    // Public transaction commands
    /// Call a contract using a public account
    MoonlightContractCall {
        /// Profile index for the public account address that will pay for the
        /// gas [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

        /// contract id of the contract to call
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
    /// Convert shielded DUSK to public Dusk (the conversion will happen
    /// between the shielded and public addresses of the same profile)
    PhoenixToMoonlight {
        /// Profile index for the DUSK conversion [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

        /// Amount of DUSK to transfer to your public account
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
        /// Profile index for the public account that will be listed as the
        /// owner of the contract [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

        /// Path to the WASM contract code
        #[clap(short, long)]
        code: PathBuf,

        /// Nonce used for the deploy transaction
        #[clap(short, long)]
        deploy_nonce: u64,
    },

    /// Convert public DUSK to shielded Dusk (the conversion will happen
    /// between the shielded and public addresses of the same profile)
    MoonlightToPhoenix {
        /// Profile index for the DUSK conversion [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

        /// Amount of DUSK to transfer to your shielded account
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
        /// Profile index for which you want the exported keys [default: 0]
        #[clap(long)]
        profile_idx: Option<u8>,

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
    pub async fn run<'a>(
        self,
        wallet: &'a mut Wallet<WalletFile>,
        settings: &Settings,
    ) -> anyhow::Result<RunResult<'a>> {
        match self {
            Command::Balance { address, spendable } => {
                let addr = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = wallet.find_index(&addr)?;
                match addr {
                    Address::Public { .. } => Ok(RunResult::MoonlightBalance(
                        wallet.get_moonlight_balance(addr_idx).await?,
                    )),
                    Address::Shielded { .. } => {
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

                        let balance =
                            wallet.get_phoenix_balance(addr_idx).await?;
                        Ok(RunResult::PhoenixBalance(balance, spendable))
                    }
                }
            }
            Command::Profiles { new } => {
                if new {
                    if wallet.profiles().len() >= MAX_PROFILES {
                        println!(
                            "Cannot create more profiles, this wallet only supports up to {MAX_PROFILES} profiles. You have {} profiles already.", wallet.profiles().len()
                        );
                        std::process::exit(0);
                    }

                    let new_addr_idx = wallet.add_profile();
                    wallet.save()?;

                    Ok(RunResult::Profile((
                        new_addr_idx,
                        &wallet.profiles()[new_addr_idx as usize],
                    )))
                } else {
                    let profiles = wallet.profiles();

                    Ok(RunResult::Profiles(profiles))
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
                        addr.same_protocol(&rcvr)?;
                        wallet.find_index(&addr)?
                    }
                    None => 0,
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let memo = memo.filter(|m| !m.trim().is_empty());
                let tx = match rcvr {
                    Address::Shielded { .. } => {
                        wallet.sync().await?;
                        let rcvr_pk = rcvr.shielded_address()?;
                        wallet
                            .phoenix_transfer(
                                sender_idx, rcvr_pk, memo, amt, gas,
                            )
                            .await?
                    }
                    Address::Public { .. } => {
                        let rcvr_pk = rcvr.public_address()?;
                        wallet
                            .moonlight_transfer(
                                sender_idx, rcvr_pk, memo, amt, gas,
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
                let addr_idx = wallet.find_index(&address)?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_stake(addr_idx, amt, gas).await
                    }
                    Address::Public { .. } => {
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
                let profile_idx = wallet.find_index(&address)?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_unstake(profile_idx, gas).await
                    }
                    Address::Public { .. } => {
                        wallet.moonlight_unstake(profile_idx, gas).await
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
                let addr_idx = wallet.find_index(&address)?;
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded { .. } => {
                        wallet.sync().await?;
                        wallet.phoenix_stake_withdraw(addr_idx, gas).await
                    }
                    Address::Public { .. } => {
                        wallet.moonlight_stake_withdraw(addr_idx, gas).await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::StakeInfo {
                profile_idx,
                reward,
            } => {
                let profile_idx = profile_idx.unwrap_or_default();
                let stake_info = wallet
                    .stake_info(profile_idx)
                    .await?
                    .ok_or(Error::NotStaked)?;

                Ok(RunResult::StakeInfo(stake_info, reward))
            }
            Command::Export {
                profile_idx,
                dir,
                name,
            } => {
                let pwd = prompt::request_auth(
                    "Provide a password for your provisioner keys",
                    &settings.password,
                    wallet.get_file_version()?,
                )?;
                let profile_idx = profile_idx.unwrap_or_default();

                let (pub_key, key_pair) = wallet.export_provisioner_keys(
                    profile_idx,
                    &dir,
                    name,
                    &pwd,
                )?;

                Ok(RunResult::ExportedKeys(pub_key, key_pair))
            }
            Command::PhoenixHistory { profile_idx } => {
                wallet.sync().await?;
                let profile_idx = profile_idx.unwrap_or_default();
                let notes = wallet.get_all_notes(profile_idx).await?;

                let transactions =
                    history::transaction_from_notes(settings, notes).await?;

                Ok(RunResult::PhoenixHistory(transactions))
            }
            Command::PhoenixToMoonlight {
                profile_idx,
                gas_limit,
                gas_price,
                amt,
            } => {
                wallet.sync().await?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let profile_idx = profile_idx.unwrap_or_default();

                let tx =
                    wallet.phoenix_to_moonlight(profile_idx, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightToPhoenix {
                profile_idx,
                amt,
                gas_limit,
                gas_price,
            } => {
                wallet.sync().await?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let profile_idx = profile_idx.unwrap_or_default();

                let tx =
                    wallet.moonlight_to_phoenix(profile_idx, amt, gas).await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::PhoenixContractCall {
                profile_idx,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
            } => {
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let profile_idx = profile_idx.unwrap_or_default();

                let contract_id: [u8; CONTRACT_ID_BYTES] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name, &fn_args)
                    .map_err(|_| Error::Rkyv)?;

                let tx = wallet
                    .phoenix_execute(
                        profile_idx,
                        Dusk::from(0),
                        gas,
                        call.into(),
                    )
                    .await?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::MoonlightContractCall {
                profile_idx,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
            } => {
                let gas = Gas::new(gas_limit).with_price(gas_price);
                let profile_idx = profile_idx.unwrap_or_default();

                let contract_id: [u8; 32] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name, &fn_args)
                    .map_err(|_| Error::Rkyv)?;

                let tx = wallet
                    .moonlight_execute(
                        profile_idx,
                        Dusk::from(0),
                        Dusk::from(0),
                        gas,
                        call.into(),
                    )
                    .await?;

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
                let address = match address {
                    Some(addr) => wallet.claim_as_address(addr)?,
                    None => wallet.default_address(),
                };
                let addr_idx = wallet.find_index(&address)?;

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }
                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded { .. } => {
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
                    Address::Public { .. } => {
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
                profile_idx,
                code,
                deploy_nonce,
            } => {
                let profile_idx = profile_idx.unwrap_or_default();

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }

                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let contract_id =
                    wallet.get_contract_id(profile_idx, code, deploy_nonce)?;

                Ok(RunResult::ContractId(contract_id))
            }
            Command::Create { .. } => Ok(RunResult::Create()),
            Command::Restore { .. } => Ok(RunResult::Restore()),
            Command::Settings => Ok(RunResult::Settings()),
        }
    }
}

/// Possible results of running a command in interactive mode
pub enum RunResult<'a> {
    Tx(BlsScalar),
    PhoenixBalance(BalanceInfo, bool),
    MoonlightBalance(Dusk),
    StakeInfo(StakeData, bool),
    Profile((u8, &'a Profile)),
    Profiles(&'a Vec<Profile>),
    ContractId([u8; CONTRACT_ID_BYTES]),
    ExportedKeys(PathBuf, PathBuf),
    Create(),
    Restore(),
    Settings(),
    PhoenixHistory(Vec<TransactionHistory>),
}

impl fmt::Display for RunResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RunResult::*;
        match self {
            PhoenixBalance(balance, _) => {
                let total = Dusk::from(balance.value);
                let spendable = Dusk::from(balance.spendable);
                write!(
                    f,
                    "> Total shielded balance: {total} DUSK\n\
                     > Maximum spendable per TX: {spendable} DUSK",
                )
            }
            MoonlightBalance(balance) => {
                write!(f, "> Total public balance: {balance} DUSK")
            }
            Profile((profile_idx, profile)) => {
                write!(
                    f,
                    "> {}\n>   {}\n>   {}",
                    crate::Profile::index_string(*profile_idx),
                    profile.shielded_address_string(),
                    profile.public_account_string(),
                )
            }
            Profiles(addrs) => {
                let profiles_string = addrs
                    .iter()
                    .enumerate()
                    .map(|(profile_idx, profile)| {
                        format!(
                            "> {}\n>   {}\n>   {}\n",
                            crate::Profile::index_string(profile_idx as u8),
                            profile.shielded_address_string(),
                            profile.public_account_string(),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                write!(f, "{}", profiles_string,)
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
