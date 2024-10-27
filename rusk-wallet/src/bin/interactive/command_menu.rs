// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use execution_core::transfer::data::MAX_MEMO_SIZE;
use requestty::Question;
use rusk_wallet::{
    currency::Dusk, gas, Address, Wallet, MAX_FUNCTION_NAME_SIZE,
};

use crate::{prompt, settings::Settings, Command, Menu, WalletFile};

use super::ProfileOp;

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

/// Allows the user to choose the operation to perform for the
/// selected profile
pub(crate) fn online(
    profile_idx: u8,
    wallet: &Wallet<WalletFile>,
    phoenix_balance: Dusk,
    moonlight_balance: Dusk,
    settings: &Settings,
) -> anyhow::Result<ProfileOp> {
    let cmd_menu = Menu::new()
        .add(MenuItem::History, "Show Transactions History")
        .add(MenuItem::Transfer, "Transfer Dusk")
        .add(MenuItem::Unshield, "Convert Shielded Dusk to Public Dusk")
        .add(MenuItem::Shield, "Convert Public Dusk to Shielded Dusk")
        .add(MenuItem::StakeInfo, "Check Existing Stake")
        .add(MenuItem::Stake, "Stake")
        .add(MenuItem::Unstake, "Unstake")
        .add(MenuItem::Withdraw, "Withdraw Stake Reward")
        .add(MenuItem::ContractCall, "Call a Contract")
        .add(MenuItem::ContractDeploy, "Deploy a Contract")
        .add(MenuItem::CalculateContractId, "Calculate Contract ID")
        .add(MenuItem::Export, "Export Provisioner Key-Pair")
        .separator()
        .add(MenuItem::Back, "Back")
        .separator();

    let q = Question::select("theme")
        .message("What do you like to do?")
        .choices(cmd_menu.clone())
        .page_size(20)
        .build();

    let answer = requestty::prompt_one(q)?;
    let cmd = cmd_menu.answer(&answer).to_owned();

    let res = match cmd {
        MenuItem::Transfer => {
            let rcvr = prompt::request_rcvr_addr("recipient")?;
            let (sender, balance) = match &rcvr {
                Address::Shielded(_) => {
                    (wallet.shielded_address(profile_idx)?, phoenix_balance)
                }
                Address::Public(_) => {
                    (wallet.public_address(profile_idx)?, moonlight_balance)
                }
            };

            let memo = Some(prompt::request_str("memo", MAX_MEMO_SIZE)?);
            let amt = if memo.is_some() {
                prompt::request_optional_token_amt("transfer", balance)
            } else {
                prompt::request_token_amt("transfer", balance)
            }?;

            ProfileOp::Run(Box::new(Command::Transfer {
                sender: Some(sender),
                rcvr,
                amt,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_TRANSFER,
                )?,
                memo,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        MenuItem::Stake => {
            let (addr, balance) = match prompt::request_transaction_model()? {
                prompt::TransactionModel::Shielded => {
                    (wallet.shielded_address(profile_idx)?, phoenix_balance)
                }
                prompt::TransactionModel::Public => {
                    (wallet.public_address(profile_idx)?, moonlight_balance)
                }
            };
            ProfileOp::Run(Box::new(Command::Stake {
                address: Some(addr),
                amt: prompt::request_stake_token_amt(balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        MenuItem::Unstake => {
            let addr = match prompt::request_transaction_model()? {
                prompt::TransactionModel::Shielded => {
                    wallet.shielded_address(profile_idx)
                }
                prompt::TransactionModel::Public => {
                    wallet.public_address(profile_idx)
                }
            }?;
            ProfileOp::Run(Box::new(Command::Unstake {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }

        MenuItem::Withdraw => {
            let addr = match prompt::request_transaction_model()? {
                prompt::TransactionModel::Shielded => {
                    wallet.shielded_address(profile_idx)
                }
                prompt::TransactionModel::Public => {
                    wallet.public_address(profile_idx)
                }
            }?;
            ProfileOp::Run(Box::new(Command::Withdraw {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        MenuItem::ContractDeploy => {
            let addr = match prompt::request_transaction_model()? {
                prompt::TransactionModel::Shielded => {
                    wallet.shielded_address(profile_idx)
                }
                prompt::TransactionModel::Public => {
                    wallet.public_address(profile_idx)
                }
            }?;
            ProfileOp::Run(Box::new(Command::ContractDeploy {
                address: Some(addr),
                code: prompt::request_contract_code()?,
                init_args: prompt::request_bytes("init arguments")?,
                deploy_nonce: prompt::request_nonce()?,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_DEPLOYMENT,
                )?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        MenuItem::ContractCall => {
            let addr = match prompt::request_transaction_model()? {
                prompt::TransactionModel::Shielded => {
                    wallet.shielded_address(profile_idx)
                }
                prompt::TransactionModel::Public => {
                    wallet.public_address(profile_idx)
                }
            }?;
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
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
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
        MenuItem::Shield => ProfileOp::Run(Box::new(Command::Shield {
            profile_idx: Some(profile_idx),
            amt: prompt::request_token_amt("convert", moonlight_balance)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
            gas_price: prompt::request_gas_price()?,
        })),
        MenuItem::Unshield => ProfileOp::Run(Box::new(Command::Unshield {
            profile_idx: Some(profile_idx),
            amt: prompt::request_token_amt("convert", phoenix_balance)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
            gas_price: prompt::request_gas_price()?,
        })),
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
    let cmd_menu = Menu::new()
        .separator()
        .add(MenuItem::Export, "Export provisioner key-pair")
        .separator()
        .add(MenuItem::Back, "Back");

    let q = Question::select("theme")
        .message("[OFFLINE] What would you like to do?")
        .choices(cmd_menu.clone())
        .build();

    let answer = requestty::prompt_one(q)?;
    let cmd = cmd_menu.answer(&answer).to_owned();

    let res = match cmd {
        MenuItem::Export => ProfileOp::Run(Box::new(Command::Export {
            profile_idx: Some(profile_idx),
            name: None,
            dir: prompt::request_dir(
                "export keys",
                settings.wallet_dir.clone(),
            )?,
        })),
        MenuItem::Back => ProfileOp::Back,
        _ => unreachable!(),
    };
    Ok(res)
}
