// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod driver_upload;
mod history;

use dusk_core::transfer::data::BlobData;
pub use history::TransactionHistory;
use zeroize::Zeroize;

#[cfg(all(test, feature = "e2e-test"))]
mod tests;

use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use aes_gcm::AeadCore;
use aes_gcm::Aes256Gcm;
use bip39::{Language, Mnemonic, MnemonicType};
use clap::Subcommand;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::stake::StakeData;
use dusk_core::transfer::data::ContractCall;
use dusk_core::BlsScalar;
use rand::rngs::OsRng;
use rand::RngCore;
use rusk_wallet::currency::{Dusk, Lux};
use rusk_wallet::dat::{self, LATEST_VERSION};
use rusk_wallet::gas::{
    Gas, DEFAULT_LIMIT_CALL, DEFAULT_LIMIT_DEPLOYMENT, DEFAULT_LIMIT_TRANSFER,
    DEFAULT_PRICE, MIN_PRICE_DEPLOYMENT,
};
use rusk_wallet::{
    Address, Error, Profile, Wallet, EPOCH, IV_SIZE, MAX_PROFILES, SALT_SIZE,
};
use wallet_core::BalanceInfo;

use crate::io::prompt;
use crate::prompt::Prompt;
use crate::settings::Settings;
use crate::{WalletFile, WalletPath};

pub(crate) use self::history::BalanceType;

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

    /// [DEPRECATED] Use `claim-rewards` instead
    #[command(hide = true)]
    Withdraw {
        /// Address from which to make the withdraw request [default:
        /// first address]
        #[arg(short, long)]
        address: Option<Address>,

        /// Amount of rewards to withdraw from the stake contract. If the
        /// reward is not provided, all the rewards are withdrawn at
        /// once
        #[arg(short, long)]
        reward: Option<Dusk>,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
    },

    /// Claim accumulated stake rewards
    ClaimRewards {
        /// Address from which to make the claim rewards request [default:
        /// first address]
        #[arg(short, long)]
        address: Option<Address>,

        /// Amount of rewards to claim from the stake contract. If the
        /// reward is not provided, all the rewards are claimed at
        /// once
        #[arg(short, long)]
        reward: Option<Dusk>,

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

    /// Deploy a driver
    DriverDeploy {
        /// Profile index for the public account that will be listed as the
        /// owner of the contract [default: 0]
        #[arg(long)]
        profile_idx: Option<u8>,
        /// Path to the WASM driver code
        #[arg(short, long)]
        code: PathBuf,
        /// Contract id of the driver's contract
        #[arg(short, long, value_parser = parse_hex)]
        contract_id: std::vec::Vec<u8>, /* Fully qualify it due to https://github.com/clap-rs/clap/issues/4481#issuecomment-1314475143 */
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

    /// Send a Blob transaction
    Blob {
        /// Address that pays the gas for the blob transaction [default: first]
        #[arg(short, long)]
        address: Option<Address>,

        /// Paths to the files containing the blob data
        #[arg(long, required = true)]
        blobs: Vec<PathBuf>,

        /// Max amount of gas for this transaction
        #[arg(short = 'l', long, default_value_t = DEFAULT_LIMIT_CALL)]
        gas_limit: u64,

        /// Price you're going to pay for each gas unit (in LUX)
        #[arg(short = 'p', long, default_value_t = DEFAULT_PRICE)]
        gas_price: Lux,
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
        let is_withdraw = matches!(self, Command::Withdraw { .. });
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
                reward,
                gas_limit,
                gas_price,
            }
            | Command::ClaimRewards {
                address,
                reward,
                gas_limit,
                gas_price,
            } => {
                if is_withdraw {
                    println!("`withdraw` is deprecated. Please use `claim_rewards` instead.");
                }
                let address = address.unwrap_or(wallet.default_address());
                let addr_idx = wallet.find_index(&address)?;

                let gas = Gas::new(gas_limit).with_price(gas_price);
                let tx = match address {
                    Address::Shielded(_) => {
                        wallet.sync().await?;
                        wallet
                            .phoenix_claim_rewards(addr_idx, reward, gas)
                            .await
                    }
                    Address::Public(_) => {
                        wallet
                            .moonlight_claim_rewards(addr_idx, reward, gas)
                            .await
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
                let mut pwd = match export_pwd {
                    Some(pwd) => pwd,
                    None => match settings.password.as_ref() {
                        Some(p) => p.to_string(),
                        None => prompt::ask_pwd(
                            "Provide a password for your provisioner keys",
                        )?,
                    },
                };

                let profile_idx = profile_idx.unwrap_or_default();

                let res = wallet.export_provisioner_keys(
                    profile_idx,
                    &dir,
                    name,
                    &pwd,
                );

                pwd.zeroize();

                let (pub_key, key_pair) = res?;

                Ok(RunResult::ExportedKeys(pub_key, key_pair))
            }
            Command::History { profile_idx } => {
                let profile_idx = profile_idx.unwrap_or_default();

                wallet.sync().await?;
                let notes = wallet.get_all_notes(profile_idx)?;
                let address = wallet.public_address(profile_idx)?;

                let mut history =
                    history::transaction_from_notes(settings, notes, &address)
                        .await?;

                match history::moonlight_history(settings, address).await {
                    Ok(mut moonlight_history) => {
                        history.append(&mut moonlight_history)
                    }
                    Err(err) => tracing::error!(
                        "Failed to fetch archive history with error: {err}"
                    ),
                }

                history.sort_by_key(|th| th.height());
                Ok(RunResult::History(history))
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
            Self::DriverDeploy {
                profile_idx,
                code,
                contract_id,
            } => {
                let profile_idx = profile_idx.unwrap_or_default();
                let contract_id_bytes: [u8; CONTRACT_ID_BYTES] = contract_id
                    .try_into()
                    .map_err(|_| Error::InvalidContractId)?;
                let contract_id = ContractId::from_bytes(contract_id_bytes);
                driver_upload::driver_upload(
                    &code,
                    &contract_id,
                    wallet,
                    profile_idx,
                )
                .await?;
                Ok(RunResult::DriverDeployResult(contract_id))
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
            Self::Blob {
                address,
                blobs,
                gas_limit,
                gas_price,
            } => {
                let address = address.unwrap_or(wallet.default_address());
                address.public_key().map_err(|_| {
                    Error::Blob(
                        "Blob is unsupported for Shielded Addresses"
                            .to_string(),
                    )
                })?;
                let addr_idx = wallet.find_index(&address)?;
                let gas = Gas::new(gas_limit).with_price(gas_price);

                let mut tx_blobs = vec![];
                for path in blobs {
                    let mut blob =
                        std::fs::read(path.as_path()).map_err(|e| {
                            Error::Blob(format!("Invalid path {path:?}: {e:?}"))
                        })?;
                    if blob.starts_with(b"0x") {
                        blob = hex::decode(&blob[2..]).map_err(|e| {
                            Error::Blob(format!(
                                "Invalid hex in {path:?}: {e:?}"
                            ))
                        })?;
                    }

                    let blob = BlobData::from_datapart(&blob, None)
                        .map_err(|e| Error::Blob(format!("{e}")))?;
                    tx_blobs.push(blob);
                }

                let tx = wallet
                    .moonlight_execute(
                        addr_idx,
                        Dusk::from(0),
                        Dusk::from(0),
                        gas,
                        Some(tx_blobs),
                    )
                    .await?;
                Ok(RunResult::Tx(tx.hash()))
            }
            Command::Create { .. } => Ok(RunResult::Create()),
            Command::Restore { .. } => Ok(RunResult::Restore()),
            Command::Settings => Ok(RunResult::Settings()),
        }
    }

    pub fn max_deduction(&self) -> (BalanceType, Dusk) {
        match self {
            Command::Shield { amt, .. }
            | Command::Unshield { amt, .. }
            | Command::ContractCall { deposit: amt, .. }
            | Command::Stake { amt, .. }
            | Command::Transfer { amt, .. } => {
                let (bal_type, fee) = self.max_fee();
                (bal_type, fee + *amt)
            }
            Command::Balance { .. }
            | Command::Blob { .. }
            | Command::CalculateContractId { .. }
            | Command::ClaimRewards { .. }
            | Command::Create { .. }
            | Command::Restore { .. }
            | Command::Settings
            | Command::Export { .. }
            | Command::History { .. }
            | Command::Profiles { .. }
            | Command::Withdraw { .. }
            | Command::StakeInfo { .. }
            | Command::Unstake { .. }
            | Command::ContractDeploy { .. }
            | Command::DriverDeploy { .. } => self.max_fee(),
        }
    }

    pub fn max_fee(&self) -> (BalanceType, Dusk) {
        match self {
            Command::Blob {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::Withdraw {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::ClaimRewards {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::ContractDeploy {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::ContractCall {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::Stake {
                address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::Transfer {
                sender: address,
                gas_limit,
                gas_price,
                ..
            }
            | Command::Unstake {
                address,
                gas_limit,
                gas_price,
                ..
            } => match address {
                Some(Address::Public(_)) | None => {
                    (BalanceType::Public, Dusk::from(gas_limit * gas_price))
                }
                Some(Address::Shielded(_)) => {
                    (BalanceType::Shielded, Dusk::from(gas_limit * gas_price))
                }
            },
            Command::Shield {
                gas_limit,
                gas_price,
                ..
            } => (BalanceType::Public, Dusk::from(gas_limit * gas_price)),
            Command::Unshield {
                gas_limit,
                gas_price,
                ..
            } => (BalanceType::Shielded, Dusk::from(gas_limit * gas_price)),
            Command::Settings
            | Command::CalculateContractId { .. }
            | Command::Create { .. }
            | Command::Restore { .. }
            | Command::StakeInfo { .. }
            | Command::Profiles { .. }
            | Command::Balance { .. }
            | Command::History { .. }
            | Command::Export { .. }
            | Command::DriverDeploy { .. } => {
                (BalanceType::Public, Dusk::from(0))
            }
        }
    }

    pub(crate) fn run_create(
        skip_recovery: bool,
        seed_file: &Option<PathBuf>,
        password: &Option<String>,
        wallet_path: &WalletPath,
        prompter: &dyn Prompt,
    ) -> anyhow::Result<Wallet<WalletFile>> {
        // create a new randomly generated mnemonic phrase
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let salt = gen_salt();
        let iv = gen_iv();
        // ask user for a password to secure the wallet
        // latest version is used for dat file
        let key = prompt::derive_key_from_new_password(
            password,
            Some(&salt),
            dat::FileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            prompter,
        )?;

        match (skip_recovery, seed_file) {
            (_, Some(file)) => {
                let mut file = File::create(file)?;
                file.write_all(mnemonic.phrase().as_bytes())?
            }
            // skip phrase confirmation if explicitly
            (false, _) => prompt::confirm_mnemonic_phrase(&mnemonic)?,
            _ => {}
        }

        // create wallet
        let mut w = Wallet::new(mnemonic)?;

        w.save_to(WalletFile {
            path: wallet_path.clone(),
            aes_key: key,
            salt: Some(salt),
            iv: Some(iv),
        })
        .inspect_err(|_| w.close())?;

        Ok(w)
    }

    pub fn run_restore_from_seed(
        wallet_path: &WalletPath,
        prompter: &dyn Prompt,
    ) -> anyhow::Result<Wallet<WalletFile>> {
        // ask user for 12-word mnemonic phrase
        let phrase = prompt::request_mnemonic_phrase(prompter)?;
        let salt = gen_salt();
        let iv = gen_iv();
        // ask user for a password to secure the wallet, create the latest
        // wallet file from the seed
        let key = prompt::derive_key_from_new_password(
            &None,
            Some(&salt),
            dat::FileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            prompter,
        )?;
        // create and store the recovered wallet
        let mut w = Wallet::new(phrase)?;
        let path = wallet_path.clone();
        w.save_to(WalletFile {
            path,
            aes_key: key,
            salt: Some(salt),
            iv: Some(iv),
        })
        .inspect_err(|_| w.close())?;
        Ok(w)
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
    DriverDeployResult(ContractId),
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
            DriverDeployResult(contract_id) => {
                writeln!(
                    f,
                    "Driver deployed for contract: {}",
                    hex::encode(contract_id.to_bytes())
                )?;
                Ok(())
            }
            Create() | Restore() | Settings() => unreachable!(),
        }
    }
}

pub(crate) fn gen_salt() -> [u8; SALT_SIZE] {
    let mut salt = [0; SALT_SIZE];
    let mut rng = OsRng;
    rng.fill_bytes(&mut salt);
    salt
}

pub(crate) fn gen_iv() -> [u8; IV_SIZE] {
    let iv = Aes256Gcm::generate_nonce(OsRng);
    iv.into()
}
