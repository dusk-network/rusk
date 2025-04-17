// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod history;

pub use history::TransactionHistory;

use std::fmt;
use std::path::PathBuf;

use clap::Subcommand;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::stake::StakeData;
use dusk_core::transfer::data::ContractCall;
use dusk_core::BlsScalar;
use rusk_wallet::currency::{Dusk, Lux};
use rusk_wallet::gas::{
    Gas, DEFAULT_LIMIT_CALL, DEFAULT_LIMIT_DEPLOYMENT, DEFAULT_LIMIT_TRANSFER,
    DEFAULT_PRICE, MIN_PRICE_DEPLOYMENT,
};
use rusk_wallet::{Address, Error, Profile, Wallet, EPOCH, MAX_PROFILES};
use wallet_core::BalanceInfo;

use crate::io::prompt;
use crate::settings::Settings;
use crate::{WalletFile, WalletPath};

/// Commands that can be run against the Dusk wallet
#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub(crate) enum Command {
    /// Create a new wallet
    Create {
        /// Skip wallet mnemonic phrase (useful for headless wallet creation)
        #[arg(long)]
        skip_recovery: bool,

        /// Save mnemonic phrase to file (useful for headless wallet creation)
        #[arg(long)]
        seed_file: Option<PathBuf>,
    },

    /// Restore a lost wallet
    Restore {
        /// Set the wallet .dat file to restore from
        #[arg(short, long)]
        file: Option<WalletPath>,
    },

    /// Check your current balance
    Balance {
        /// Address
        #[arg(long)]
        address: Option<Address>,
        /// Check maximum spendable balance
        #[arg(long)]
        spendable: bool,
    },

    /// List your existing profiles and generate new ones
    Profiles {
        /// Create new profile
        #[arg(short, long)]
        new: bool,
    },

    /// Show address transaction history
    History {
        /// Profile index for which you want to see the history
        #[arg(long)]
        profile_idx: Option<u8>,
    },

    /// Send DUSK through the network
    Transfer {
        /// Address from which to send DUSK [default: first address]
        #[arg(long)]
        sender: Option<Address>,

        /// Receiver address
        #[arg(short, long)]
        rcvr: Address,

        /// Amount of DUSK to send
        #[arg(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_TRANSFER)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,

        /// Optional memo to attach to the transaction
        #[arg(long)]
        memo: Option<String>,
    },

    /// Convert shielded DUSK to public DUSK
    Unshield {
        /// Profile index for the DUSK conversion [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,

        /// Amount of DUSK to transfer to your public account
        #[arg(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Convert public DUSK to shielded DUSK
    Shield {
        /// Profile index for the DUSK conversion [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,

        /// Amount of DUSK to transfer to your shielded account
        #[arg(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Check your stake information
    StakeInfo {
        /// Profile index for the public account address to stake from
        /// [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,

        /// Check accumulated reward
        #[arg(long)]
        reward: bool,
    },

    /// Stake DUSK
    Stake {
        /// Address from which to stake DUSK [default: first address]
        #[arg(long)]
        address: Option<Address>,

        /// Owner of the stake [default: same Public address of the stake]
        #[arg(long)]
        owner: Option<Address>,

        /// Amount of DUSK to stake
        #[arg(short, long)]
        amt: Dusk,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Unstake DUSK
    Unstake {
        /// Address from which to make the unstake request [default: first
        /// address]
        #[arg(short, long)]
        address: Option<Address>,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Withdraw accumulated rewards for a stake key
    Withdraw {
        /// Address from which to make the withdraw request [default:
        /// first address]
        #[arg(short, long)]
        address: Option<Address>,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Call a contract
    ContractCall {
        /// Address that pays the gas for the contract call [default: first]
        #[arg(short, long)]
        address: Option<Address>,

        /// Contract id of the contract to call
        #[arg(short, long, value_parser = parse_hex)]
        contract_id: std::vec::Vec<u8>, /* Fully qualify it due to https://github.com/clap-rs/clap/issues/4481#issuecomment-1314475143 */

        /// Function name to call
        #[arg(short = 'n', long)]
        fn_name: String,

        /// Function arguments for this call (hex encoded rkyv serialized data)
        #[arg(short = 'f', long, value_parser = parse_hex)]
        fn_args: std::vec::Vec<u8>, /* Fully qualify it due to https://github.com/clap-rs/clap/issues/4481#issuecomment-1314475143 */

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,

        /// Amount of DUSK to deposit to the contract
        #[arg(long, default_value_t = Dusk::MIN)]
        deposit: Dusk,
    },

    /// Deploy a contract
    ContractDeploy {
        /// Address that will pay for the gas to deploy the contract [default:
        /// first]
        #[arg(short, long)]
        address: Option<Address>,

        /// Path to the WASM contract code
        #[arg(short, long)]
        code: PathBuf,

        /// Arguments for init function (hex encoded rkyv serialized data)
        #[arg(short, long, default_value = "", value_parser = parse_hex)]
        init_args: std::vec::Vec<u8>, /* Fully qualify it due to https://github.com/clap-rs/clap/issues/4481#issuecomment-1314475143 */

        /// Nonce used for the deploy transaction
        #[arg(short, long)]
        deploy_nonce: u64,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_DEPLOYMENT)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = MIN_PRICE_DEPLOYMENT)]
        gas_price: Lux,
    },

    /// Calculate a contract id
    CalculateContractId {
        /// Profile index for the public account that will be listed as the
        /// owner of the contract [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,

        /// Path to the WASM contract code
        #[arg(short, long)]
        code: PathBuf,

        /// Nonce used for the deploy transaction
        #[arg(short, long)]
        deploy_nonce: u64,
    },

    /// Export BLS provisioner key-pair
    Export {
        /// Profile index for which you want the exported keys [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,

        /// Output directory for the exported keys
        #[arg(short, long)]
        dir: PathBuf,

        /// Name of the files exported [default: staking-address]
        #[arg(short, long)]
        name: Option<String>,

        /// Password for the exported keys [default: env(RUSK_WALLET_PWD)]
        #[arg(short, long, env = "RUSK_WALLET_EXPORT_PWD")]
        export_pwd: Option<String>,
    },

    /// Show current settings
    Settings,
}

fn parse_hex(hex_str: &str) -> Result<Vec<u8>, String> {
    hex::decode(hex_str).map_err(|e| e.to_string())
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
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                match address {
                    Address::Public(_) => Ok(RunResult::MoonlightBalance(
                        wallet.get_moonlight_balance(addr_idx).await?,
                    )),
                    Address::Shielded(_) => {
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
                        addr.same_transaction_model(&rcvr)?;
                        wallet.find_index(&addr)?
                    }
                    None => 0,
                };

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let memo = memo.filter(|m| !m.trim().is_empty());
                let tx = match rcvr {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        let rcvr_pk = rcvr.shielded_key()?;
                        wallet
                            .phoenix_transfer(
                                sender_idx, rcvr_pk, memo, amt, gas,
                            )
                            .await?
                    }
                    Address::Public(_) => {
                        let rcvr_pk = rcvr.public_key()?;
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
                owner,
                amt,
                gas_limit,
                gas_price,
            } => {
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;
                let owner_idx =
                    owner.map(|owner| wallet.find_index(&owner)).transpose()?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_stake(addr_idx, owner_idx, amt, gas)
                            .await
                    }
                    Address::Public(_) => {
                        wallet
                            .moonlight_stake(addr_idx, owner_idx, amt, gas)
                            .await
                    }
                }?;

                Ok(RunResult::Tx(tx.hash()))
            }
            Command::Unstake {
                address,
                gas_limit,
                gas_price,
            } => {
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        wallet.phoenix_unstake(addr_idx, gas).await
                    }
                    Address::Public(_) => {
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
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        wallet.phoenix_stake_withdraw(addr_idx, gas).await
                    }
                    Address::Public(_) => {
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
                export_pwd,
            } => {
                let pwd = match export_pwd {
                    Some(pwd) => pwd,
                    None => match settings.password.as_ref() {
                        Some(p) => p.to_string(),
                        None => prompt::ask_pwd(
                            "Provide a password for your provisioner keys",
                        )?,
                    },
                };

                let profile_idx = profile_idx.unwrap_or_default();

                let (pub_key, key_pair) = wallet.export_provisioner_keys(
                    profile_idx,
                    &dir,
                    name,
                    &pwd,
                )?;

                Ok(RunResult::ExportedKeys(pub_key, key_pair))
            }
            Command::History { profile_idx } => {
                let profile_idx = profile_idx.unwrap_or_default();

                wallet.sync().await?;
                let notes = wallet.get_all_notes(profile_idx)?;
                let address = wallet.public_address(profile_idx)?;

                let mut phoenix_history =
                    history::transaction_from_notes(settings, notes).await?;

                if let Ok(mut moonlight_history) =
                    history::moonlight_history(settings, address).await
                {
                    phoenix_history.append(&mut moonlight_history);
                } else {
                    tracing::error!("Cannot fetch archive history");
                }

                Ok(RunResult::History(phoenix_history))
            }
            Command::Unshield {
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
            Command::Shield {
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
            Command::ContractCall {
                address,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
                deposit,
            } => {
                let gas = Gas::new(gas_limit).with_price(gas_price);

                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                let contract_id: [u8; CONTRACT_ID_BYTES] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;

                let call = ContractCall::new(contract_id, fn_name)
                    .with_raw_args(fn_args);

                let tx = match address {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_execute(
                                addr_idx,
                                deposit,
                                gas,
                                call.into(),
                            )
                            .await
                    }
                    Address::Public(_) => {
                        wallet
                            .moonlight_execute(
                                addr_idx,
                                Dusk::from(0),
                                deposit,
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
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                if code.extension().unwrap_or_default() != "wasm" {
                    return Err(Error::InvalidWasmContractPath.into());
                }
                let code = std::fs::read(code)
                    .map_err(|_| Error::InvalidWasmContractPath)?;

                let gas = Gas::new(gas_limit).with_price(gas_price);

                let contract_id =
                    wallet.get_contract_id(addr_idx, &code, deploy_nonce)?;

                let tx = match address {
                    Address::Shielded(_) => {
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
                    Address::Public(_) => {
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

                Ok(RunResult::DeployTx(tx.hash(), contract_id.into()))
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
                    wallet.get_contract_id(profile_idx, &code, deploy_nonce)?;

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
    DeployTx(BlsScalar, ContractId),
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
    History(Vec<TransactionHistory>),
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
                    profile.shielded_account_string(),
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
                            profile.shielded_account_string(),
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
            DeployTx(hash, contract_id) => {
                let contract_id = hex::encode(contract_id.as_bytes());
                writeln!(f, "> Deploying contract: {contract_id}",)?;
                let hash = hex::encode(hash.to_bytes());
                write!(f, "> Transaction sent: {hash}",)
            }
            StakeInfo(data, _) => {
                if let Some(amt) = data.amount {
                    let amount = Dusk::from(amt.value);
                    let locked = Dusk::from(amt.locked);
                    let eligibility = amt.eligibility;
                    let epoch = amt.eligibility / EPOCH;

                    writeln!(f, "> Eligible stake: {amount} DUSK")?;
                    writeln!(f, "> Reclaimable slashed stake: {locked} DUSK")?;
                    writeln!(f, "> Stake active from block #{eligibility} (Epoch {epoch})")?;
                } else {
                    writeln!(f, "> No active stake found for this key")?;
                }
                let faults = data.faults;
                let hard_faults = data.hard_faults;
                let rewards = Dusk::from(data.reward);

                writeln!(f, "> Slashes: {faults}")?;
                writeln!(f, "> Hard Slashes: {hard_faults}")?;
                write!(f, "> Accumulated rewards is: {rewards} DUSK")
            }
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
            History(txns) => {
                writeln!(f, "{}", TransactionHistory::header())?;
                for th in txns {
                    writeln!(f, "{th}")?;
                }
                Ok(())
            }
            Create() | Restore() | Settings() => unreachable!(),
        }
    }
}
