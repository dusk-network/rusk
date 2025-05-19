// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Display;

use dusk_core::stake::DEFAULT_MINIMUM_STAKE;
use dusk_core::transfer::data::MAX_MEMO_SIZE;
use inquire::{InquireError, Select};
use rusk_wallet::currency::Dusk;
use rusk_wallet::gas::{
    self, DEFAULT_LIMIT_CALL, DEFAULT_LIMIT_STAKE, DEFAULT_LIMIT_TRANSFER,
    DEFAULT_PRICE, GAS_PER_DEPLOY_BYTE, MIN_PRICE_DEPLOYMENT,
};
use rusk_wallet::{
    Address, Error, Wallet, MAX_FUNCTION_NAME_SIZE, MIN_CONVERTIBLE,
};

use super::ProfileOp;
use crate::settings::Settings;
use crate::{prompt, Command, WalletFile};

/// The command-menu items
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum MenuItem {
    History,
    Transfer,
    Stake,
    Unstake,
    Withdraw,
    ContractDeploy,
    ContractCall,
    Unshield,
    Shield,
    CalculateContractId,
    StakeInfo,
    Export,
    Back,
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuItem::History => write!(f, "Show Transactions History"),
            MenuItem::Transfer => write!(f, "Transfer Dusk"),
            MenuItem::Stake => write!(f, "Stake"),
            MenuItem::Unstake => write!(f, "Unstake"),
            MenuItem::Withdraw => write!(f, "Withdraw Stake Reward"),
            MenuItem::ContractDeploy => write!(f, "Deploy a Contract"),
            MenuItem::ContractCall => write!(f, "Call a Contract"),
            MenuItem::Unshield => {
                write!(f, "Convert Shielded Dusk to Public Dusk")
            }
            MenuItem::Shield => {
                write!(f, "Convert Public Dusk to Shielded Dusk")
            }
            MenuItem::CalculateContractId => write!(f, "Calculate Contract ID"),
            MenuItem::StakeInfo => write!(f, "Stake Info"),
            MenuItem::Export => write!(f, "Export Provisioner Key-Pair"),
            MenuItem::Back => write!(f, "Back"),
        }
    }
}

/// Allows the user to choose the operation to perform for the
/// selected profile
pub(crate) async fn online(
    profile_idx: u8,
    wallet: &Wallet<WalletFile>,
    phoenix_spendable: Dusk,
    moonlight_balance: Dusk,
    settings: &Settings,
) -> anyhow::Result<ProfileOp> {
    let cmd_menu = vec![
        MenuItem::History,
        MenuItem::Transfer,
        MenuItem::Unshield,
        MenuItem::Shield,
        MenuItem::StakeInfo,
        MenuItem::Stake,
        MenuItem::Unstake,
        MenuItem::Withdraw,
        MenuItem::ContractCall,
        MenuItem::ContractDeploy,
        MenuItem::CalculateContractId,
        MenuItem::Export,
        MenuItem::Back,
    ];

    let select = Select::new("What would you like to do?", cmd_menu).prompt();

    if let Err(InquireError::OperationCanceled) = select {
        return Ok(ProfileOp::Back);
    }

    let select = select?;

    let res = match select {
        MenuItem::Transfer => {
            let rcvr = prompt::request_rcvr_addr("recipient")?;

            let (sender, balance) = match &rcvr {
                Address::Shielded(_) => {
                    (wallet.shielded_account(profile_idx)?, phoenix_spendable)
                }
                Address::Public(_) => {
                    (wallet.public_address(profile_idx)?, moonlight_balance)
                }
            };

            if check_min_gas_balance(
                balance,
                DEFAULT_LIMIT_TRANSFER,
                "a transfer transaction",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let memo = Some(prompt::request_str("memo", MAX_MEMO_SIZE)?);
            let amt = if memo.is_some() {
                prompt::request_optional_token_amt("transfer", balance)
            } else {
                prompt::request_token_amt("transfer", balance)
            }?;

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            ProfileOp::Run(Box::new(Command::Transfer {
                sender: Some(sender),
                rcvr,
                amt,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_TRANSFER,
                )?,
                memo,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::Stake => {
            let (addr, balance) = pick_transaction_model(
                wallet,
                profile_idx,
                phoenix_spendable,
                moonlight_balance,
            )?;

            if check_min_gas_balance(
                balance,
                DEFAULT_LIMIT_STAKE,
                "a stake transaction",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            let stake_idx = wallet
                .find_index(&addr)
                .expect("index to exists in interactive mode");
            let stake_pk = wallet
                .public_key(stake_idx)
                .expect("public key to exists in interactive mode");

            let min_val = {
                let has_stake = wallet
                    .stake_info(stake_idx)
                    .await?
                    .map(|s| s.amount.is_some())
                    .unwrap_or_default();

                // if the user has stake then they are performing a topup
                if has_stake {
                    MIN_CONVERTIBLE
                } else {
                    DEFAULT_MINIMUM_STAKE.into()
                }
            };

            if balance < min_val {
                println!("The stake must be at least {min_val}, but your balance is only {balance}\n");
                return Ok(ProfileOp::Stay);
            }

            let owner = match wallet.find_stake_owner_account(stake_pk).await {
                Ok(account) => account,
                Err(Error::NotStaked) => {
                    let choices = wallet
                        .profiles()
                        .iter()
                        .map(|p| Address::Public(p.public_addr))
                        .collect();

                    prompt::request_address(stake_idx, choices)?
                }
                e => e?,
            };

            ProfileOp::Run(Box::new(Command::Stake {
                address: Some(addr),
                owner: Some(owner),
                amt: prompt::request_stake_token_amt(balance, min_val)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::Unstake => {
            let (addr, balance) = pick_transaction_model(
                wallet,
                profile_idx,
                phoenix_spendable,
                moonlight_balance,
            )?;

            if check_min_gas_balance(
                balance,
                DEFAULT_LIMIT_STAKE,
                "an unstake transaction",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            ProfileOp::Run(Box::new(Command::Unstake {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::Withdraw => {
            let (addr, balance) = pick_transaction_model(
                wallet,
                profile_idx,
                phoenix_spendable,
                moonlight_balance,
            )?;

            if check_min_gas_balance(
                balance,
                DEFAULT_LIMIT_STAKE,
                "a stake reward withdrawal transaction",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;
            let max_withdraw = wallet.get_stake_reward(profile_idx).await?;

            ProfileOp::Run(Box::new(Command::Withdraw {
                address: Some(addr),
                reward: Some(prompt::request_token_amt_with_default(
                    "withdraw rewards",
                    max_withdraw,
                    max_withdraw,
                )?),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::ContractDeploy => {
            let (addr, balance) = pick_transaction_model(
                wallet,
                profile_idx,
                phoenix_spendable,
                moonlight_balance,
            )?;

            // Request the contract code and determine its length
            let code = prompt::request_contract_code()?;
            let code_len = code.metadata()?.len() as u64;

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            // Calculate the effective cost for the deployment
            let gas_price = prompt::request_gas_price(
                MIN_PRICE_DEPLOYMENT,
                mempool_gas_prices,
            )?;
            let gas_limit =
                (code_len * GAS_PER_DEPLOY_BYTE) + DEFAULT_LIMIT_TRANSFER;

            if check_min_gas_balance(
                balance,
                gas_limit * gas_price,
                "the deployment of the given contract",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            ProfileOp::Run(Box::new(Command::ContractDeploy {
                address: Some(addr),
                code,
                init_args: prompt::request_init_args()?,
                deploy_nonce: prompt::request_nonce()?,
                gas_limit: prompt::request_gas_limit(gas_limit)?,
                gas_price,
            }))
        }
        MenuItem::ContractCall => {
            let (addr, balance) = pick_transaction_model(
                wallet,
                profile_idx,
                phoenix_spendable,
                moonlight_balance,
            )?;

            if check_min_gas_balance(
                balance,
                DEFAULT_LIMIT_CALL,
                "a contract call",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            ProfileOp::Run(Box::new(Command::ContractCall {
                address: Some(addr),
                contract_id: prompt::request_bytes("contract id")?,
                fn_name: prompt::request_str(
                    "function name to call",
                    MAX_FUNCTION_NAME_SIZE,
                )?,
                fn_args: prompt::request_bytes(
                    "arguments of calling function",
                )?,
                deposit: prompt::request_optional_token_amt(
                    "deposit", balance,
                )?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::History => {
            let profile_idx = Some(profile_idx);

            ProfileOp::Run(Box::new(Command::History { profile_idx }))
        }
        MenuItem::StakeInfo => ProfileOp::Run(Box::new(Command::StakeInfo {
            profile_idx: Some(profile_idx),
            reward: false,
        })),
        MenuItem::Shield => {
            if check_min_gas_balance(
                moonlight_balance,
                DEFAULT_LIMIT_CALL,
                "convert DUSK from public to shielded",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            ProfileOp::Run(Box::new(Command::Shield {
                profile_idx: Some(profile_idx),
                amt: prompt::request_token_amt("convert", moonlight_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::Unshield => {
            if check_min_gas_balance(
                phoenix_spendable,
                DEFAULT_LIMIT_CALL,
                "convert DUSK from shielded to public",
            )
            .is_err()
            {
                return Ok(ProfileOp::Stay);
            }

            let mempool_gas_prices = wallet.get_mempool_gas_prices().await?;

            ProfileOp::Run(Box::new(Command::Unshield {
                profile_idx: Some(profile_idx),
                amt: prompt::request_token_amt("convert", phoenix_spendable)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price(
                    DEFAULT_PRICE,
                    mempool_gas_prices,
                )?,
            }))
        }
        MenuItem::CalculateContractId => {
            ProfileOp::Run(Box::new(Command::CalculateContractId {
                profile_idx: Some(profile_idx),
                deploy_nonce: prompt::request_nonce()?,
                code: prompt::request_contract_code()?,
            }))
        }
        MenuItem::Export => ProfileOp::Run(Box::new(Command::Export {
            profile_idx: Some(profile_idx),
            name: None,
            dir: prompt::request_dir(
                "export keys",
                settings.wallet_dir.clone(),
            )?,
            export_pwd: None,
        })),
        MenuItem::Back => ProfileOp::Back,
    };

    Ok(res)
}

/// Allows the user to choose the operation to perform for the
/// selected profile while in offline mode
pub(crate) fn offline(
    profile_idx: u8,
    settings: &Settings,
) -> anyhow::Result<ProfileOp> {
    let cmd_menu = vec![MenuItem::Export];

    let select = Select::new("[OFFLINE] What would you like to do?", cmd_menu)
        .prompt()?;

    let res = match select {
        MenuItem::Export => ProfileOp::Run(Box::new(Command::Export {
            profile_idx: Some(profile_idx),
            name: None,
            dir: prompt::request_dir(
                "export keys",
                settings.wallet_dir.clone(),
            )?,
            export_pwd: None,
        })),
        _ => unreachable!(),
    };

    Ok(res)
}

/// Prompts the user to select a transaction model (Shielded or Public), and
/// retrieves the corresponding address and balance for the specific profile
fn pick_transaction_model(
    wallet: &Wallet<WalletFile>,
    profile_idx: u8,
    phoenix_spendable: Dusk,
    moonlight_balance: Dusk,
) -> anyhow::Result<(Address, Dusk)> {
    match prompt::request_transaction_model()? {
        prompt::TransactionModel::Shielded => {
            let addr = wallet.shielded_account(profile_idx)?;
            Ok((addr, phoenix_spendable))
        }
        prompt::TransactionModel::Public => {
            let addr = wallet.public_address(profile_idx)?;
            Ok((addr, moonlight_balance))
        }
    }
}

/// Verifies that the user's balance meets the minimum required gas for a given
/// action
fn check_min_gas_balance(
    balance: Dusk,
    min_required_gas: u64,
    action: &str,
) -> anyhow::Result<()> {
    let min_required_gas: Dusk = min_required_gas.into();
    if balance < min_required_gas {
        println!(
            "Balance too low to cover the minimum gas cost for {}.",
            action
        );
        Err(anyhow::anyhow!(
            "Balance too low to cover the minimum gas cost for {}.",
            action
        ))
    } else {
        Ok(())
    }
}
