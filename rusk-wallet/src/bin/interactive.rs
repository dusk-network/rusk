// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod command_menu;

use std::fmt::Display;

use inquire::{InquireError, Select};
use rusk_wallet::currency::Dusk;
use rusk_wallet::dat;
use rusk_wallet::{Address, Error, Profile, Wallet, WalletPath, MAX_PROFILES};

use crate::command::BalanceType;
use crate::io::prompt::{EXIT_HELP, MOVE_HELP, SELECT_HELP};
use crate::io::{self, prompt};
use crate::prompt::Prompter;
use crate::settings::Settings;
use crate::{Command, GraphQL, RunResult, WalletFile};

/// Run the interactive UX loop with a loaded wallet
pub(crate) async fn run_loop(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
) -> anyhow::Result<()> {
    loop {
        // let the user choose (or create) a profile
        let profile_index = profile_idx(wallet).await?;

        loop {
            let profile = &wallet.profiles()[profile_index as usize];
            prompt::hide_cursor()?;

            let (op, moonlight_bal, phoenix_spendable) =
                if !wallet.is_online().await {
                    println!("\r{}", profile.shielded_account_string());
                    println!("{}", profile.public_account_string());
                    println!();

                    (command_menu::offline(profile_index, settings), None, None)
                } else {
                    let is_synced = wallet.is_synced().await?;
                    // get balance for this profile
                    let moonlight_bal =
                        wallet.get_moonlight_balance(profile_index).await?;
                    let phoenix_bal =
                        wallet.get_phoenix_balance(profile_index).await?;
                    let phoenix_spendable = phoenix_bal.spendable.into();
                    let phoenix_total: Dusk = phoenix_bal.value.into();

                    // display profile information
                    // display shielded balance and keys information
                    println!("{}", profile.shielded_account_string());
                    if is_synced {
                        println!(
                            "{0: <16} - Spendable: {phoenix_spendable}",
                            "Shielded Balance",
                        );
                        println!("{0: <16} - Total:     {phoenix_total}", "",);
                    } else {
                        println!("Syncing...");
                    }
                    println!();
                    // display public balance and keys information
                    println!("{}", profile.public_account_string());
                    println!(
                        "{0: <16} - Total:     {moonlight_bal}",
                        "Public Balance",
                    );
                    println!();

                    (
                        command_menu::online(
                            profile_index,
                            wallet,
                            phoenix_spendable,
                            moonlight_bal,
                            settings,
                        )
                        .await,
                        Some(moonlight_bal),
                        Some(phoenix_spendable),
                    )
                };

            prompt::hide_cursor()?;

            // perform operations with this profile
            match op {
                Ok(ProfileOp::Run(cmd)) => {
                    if let Some(more_dusk_needed) = needs_more_dusk_to_run(
                        moonlight_bal,
                        phoenix_spendable,
                        cmd.max_deduction(),
                    ) {
                        println!("Balance is not enough to cover the transaction max fee. You need {more_dusk_needed} more Dusk.\n");
                        continue;
                    }
                    // request confirmation before running
                    let should_run = match confirm(&cmd, wallet).await {
                        Ok(run) => run,
                        Err(err) => {
                            match err.downcast_ref::<InquireError>() {
                                Some(InquireError::OperationInterrupted) => {
                                    return Err(err);
                                }
                                Some(InquireError::OperationCanceled) => (),
                                _ => println!("{err}\n"),
                            };
                            continue;
                        }
                    };
                    if should_run {
                        // run command
                        prompt::hide_cursor()?;
                        let res = match cmd.run(wallet, settings).await {
                            Ok(res) => res,
                            Err(err) => {
                                match err.downcast_ref::<InquireError>() {
                                    Some(InquireError::OperationCanceled) => (),
                                    _ => println!("{err}\n"),
                                }
                                continue;
                            }
                        };
                        prompt::show_cursor()?;

                        // output results
                        match res {
                            RunResult::Tx(hash) => {
                                let tx_id = hex::encode(hash.to_bytes());

                                // Wait for transaction confirmation
                                // from network
                                let gql = GraphQL::new(
                                    settings.state.to_string(),
                                    settings.archiver.to_string(),
                                    io::status::interactive,
                                )?;
                                gql.wait_for(&tx_id).await?;

                                if let Some(explorer) = &settings.explorer {
                                    let url = format!("{explorer}{tx_id}");
                                    println!("> URL: {url}");
                                    prompt::launch_explorer(url)?;
                                }
                            }
                            RunResult::History(ref history) => {
                                if let Err(err) =
                                    crate::prompt::tx_history_list(history)
                                {
                                    match err.downcast_ref::<InquireError>() {
                                        Some(InquireError::OperationInterrupted) => {
                                            return Err(err);
                                        },
                                        Some(InquireError::OperationCanceled) => {
                                            continue;
                                        },
                                        _ => println!("Failed to output transaction history with error {err}"),
                                    }
                                }

                                println!();
                            }
                            _ => println!("\r{}", res),
                        }
                    }
                }
                Ok(ProfileOp::Stay) => (),
                Ok(ProfileOp::Back) => {
                    break;
                }
                Err(e) => match e.downcast_ref::<InquireError>() {
                    Some(InquireError::OperationCanceled) => (),
                    Some(InquireError::OperationInterrupted) => {
                        return Err(e);
                    }
                    _ => println!("{e}\n"),
                },
            };
        }
    }
}

fn needs_more_dusk_to_run(
    moonlight_bal: Option<Dusk>,
    phoenix_spendable: Option<Dusk>,
    max_deduction: (BalanceType, Dusk),
) -> Option<Dusk> {
    match (moonlight_bal, phoenix_spendable, max_deduction) {
        (Some(spendable_amount), _, (BalanceType::Public, to_deduct))
        | (_, Some(spendable_amount), (BalanceType::Shielded, to_deduct))
            if spendable_amount < to_deduct =>
        {
            Some(to_deduct - spendable_amount)
        }
        _ => None,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum ProfileSelect<'a> {
    Index(u8, &'a Profile),
    New,
    Exit,
}

async fn profile_idx(wallet: &mut Wallet<WalletFile>) -> anyhow::Result<u8> {
    match menu_profile(wallet)? {
        ProfileSelect::Index(index, _) => Ok(index),
        ProfileSelect::New => {
            if wallet.profiles().len() >= MAX_PROFILES {
                println!(
                        "Cannot create more profiles, this wallet only supports up to {MAX_PROFILES} profiles"
                    );

                return Err(InquireError::OperationCanceled.into());
            }

            let profile_idx = wallet.add_profile();

            wallet.save()?;

            Ok(profile_idx)
        }
        ProfileSelect::Exit => Err(InquireError::OperationInterrupted.into()),
    }
}

/// Allows the user to choose a profile from the selected wallet
/// to start performing operations.
fn menu_profile(wallet: &Wallet<WalletFile>) -> anyhow::Result<ProfileSelect> {
    let mut menu_items = Vec::new();
    let profiles = wallet.profiles();

    for (index, profile) in profiles.iter().enumerate() {
        menu_items.push(ProfileSelect::Index(index as u8, profile));
    }

    let remaining_profiles =
        MAX_PROFILES.saturating_sub(wallet.profiles().len());

    // only show the option to create a new profile if we don't already have
    // `MAX_PROFILES`
    if remaining_profiles > 0 {
        menu_items.push(ProfileSelect::New);
    }

    menu_items.push(ProfileSelect::Exit);

    let help_msg = &[MOVE_HELP, SELECT_HELP, EXIT_HELP].join(", ");
    let select =
        Select::new("Your Profiles", menu_items).with_help_message(help_msg);

    Ok(select.prompt()?)
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum ProfileOp {
    Run(Box<Command>),
    Back,
    Stay,
}

/// Allows the user to load a wallet interactively
pub(crate) async fn load_wallet(
    wallet_path: &WalletPath,
    settings: &Settings,
) -> anyhow::Result<Wallet<WalletFile>> {
    let wallet_found =
        wallet_path.inner().exists().then(|| wallet_path.clone());

    let password = &settings.password;

    loop {
        // display main menu
        let wallet = match menu_wallet(wallet_found.as_ref(), settings).await? {
            MainMenu::Load(path) => {
                let (file_version, salt_and_iv) =
                    dat::read_file_version_and_salt_iv(wallet_path)?;
                let mut attempt = 1;
                loop {
                    let key = prompt::derive_key_from_password(
                        "Please enter your wallet password",
                        password,
                        salt_and_iv.map(|si| si.0).as_ref(),
                        file_version,
                    );
                    let key = match key {
                        Ok(key) => key,
                        Err(err) => break Err(err),
                    };
                    match Wallet::from_file(WalletFile {
                        path: path.clone(),
                        aes_key: key,
                        salt: salt_and_iv.map(|si| si.0),
                        iv: salt_and_iv.map(|si| si.1),
                    }) {
                        Ok(wallet) => break Ok(wallet),
                        Err(_) if attempt > 2 => {
                            Err(Error::AttemptsExhausted)?;
                        }
                        Err(_) => {
                            println!("Invalid password, please try again");
                            attempt += 1;
                        }
                    }
                }
            }
            // Use the latest binary format when creating a wallet
            MainMenu::Create => Command::run_create(
                false,
                &None,
                password,
                wallet_path,
                &Prompter,
            ),
            MainMenu::Recover => {
                Command::run_restore_from_seed(wallet_path, &Prompter)
            }
            MainMenu::Exit => std::process::exit(0),
        };

        match wallet {
            Ok(wallet) => return Ok(wallet),
            Err(err) => match err.downcast_ref::<InquireError>() {
                Some(InquireError::OperationCanceled) => {
                    println!();
                    continue;
                }
                _ => return Err(err),
            },
        };
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum MainMenu {
    Load(WalletPath),
    Create,
    Recover,
    Exit,
}

/// Allows the user to load an existing wallet, recover a lost one
/// or create a new one.
async fn menu_wallet(
    wallet_found: Option<&WalletPath>,
    settings: &Settings,
) -> anyhow::Result<MainMenu> {
    // create the wallet menu
    let mut menu_items = Vec::new();

    if let Some(wallet_path) = wallet_found {
        menu_items.push(MainMenu::Load(wallet_path.clone()));
        menu_items.push(MainMenu::Create);
        menu_items.push(MainMenu::Recover);
    } else {
        menu_items.push(MainMenu::Create);
        menu_items.push(MainMenu::Recover);
    }

    menu_items.push(MainMenu::Exit);

    let emoji_state = status_emoji(settings.check_state_con().await.is_ok());
    let emoji_prover = status_emoji(settings.check_prover_con().await.is_ok());
    let emoji_archiver =
        status_emoji(settings.check_archiver_con().await.is_ok());

    let state_status = format!("{} State: {}", emoji_state, settings.state);
    let prover_status = format!("{} Prover: {}", emoji_prover, settings.prover);
    let archiver_status =
        format!("{} Archiver: {}", emoji_archiver, settings.archiver);

    let menu = format!(
        "Welcome\n   {state_status}\n   {prover_status}\n   {archiver_status}   \nWhat would you like to do?",
    );

    // let the user choose an option
    let select = Select::new(menu.as_str(), menu_items);

    Ok(select.prompt()?)
}

/// Request user confirmation for a transfer transaction
async fn confirm(
    cmd: &Command,
    wallet: &Wallet<WalletFile>,
) -> anyhow::Result<bool> {
    match cmd {
        Command::Transfer {
            sender,
            rcvr,
            amt,
            gas_limit,
            gas_price,
            memo,
        } => {
            let sender = sender.as_ref().ok_or(Error::BadAddress)?;
            sender.same_transaction_model(rcvr)?;
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", sender.preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            if let Some(memo) = memo {
                println!("   > Memo = {memo}");
            }
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        Command::Stake {
            address,
            owner,
            amt,
            gas_limit,
            gas_price,
        } => {
            let sender = address.as_ref().ok_or(Error::BadAddress)?;
            let max_fee = gas_limit * gas_price;
            let stake_to = wallet.public_address(wallet.find_index(sender)?)?;
            let owner = owner.as_ref().unwrap_or(&stake_to);
            println!("   > Pay with {}", sender.preview());
            println!("   > Stake to {}", stake_to.preview());
            println!("   > Stake owner {}", owner.preview());
            println!("   > Amount to stake = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        Command::Unstake {
            address,
            gas_limit,
            gas_price,
        } => {
            let sender = address.as_ref().ok_or(Error::BadAddress)?;
            let unstake_from =
                wallet.public_address(wallet.find_index(sender)?)?;
            let max_fee = gas_limit * gas_price;

            println!("   > Pay with {}", sender.preview());
            println!("   > Unstake from {}", unstake_from.preview());
            println!("   > Receive stake at {}", sender.preview());
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }

        Command::ClaimRewards {
            address,
            gas_limit,
            reward,
            gas_price,
        } => {
            let sender = address.as_ref().ok_or(Error::BadAddress)?;
            let sender_index = wallet.find_index(sender)?;
            let max_fee = gas_limit * gas_price;
            let claim_from = wallet.public_address(sender_index)?;

            let total_rewards = wallet.get_stake_reward(sender_index).await?;

            // claim all rewards if no amt specified
            let reward = if let Some(claim_reward) = reward {
                claim_reward
            } else {
                &total_rewards
            };

            println!("   > Pay with {}", sender.preview());
            println!("   > Claim rewards from {}", claim_from.preview());
            println!("   > Receive rewards at {}", sender.preview());
            println!("   > Amount claiming {} DUSK", reward);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        Command::ContractDeploy {
            address,
            code,
            init_args,
            deploy_nonce,
            gas_limit,
            gas_price,
        } => {
            let sender = address.as_ref().ok_or(Error::BadAddress)?;
            let sender_index = wallet.find_index(sender)?;
            let code_len = code.metadata()?.len();
            let max_fee = gas_limit * gas_price;
            let code_bytes = std::fs::read(code)?;

            let contract_id = wallet.get_contract_id(
                sender_index,
                &code_bytes,
                *deploy_nonce,
            )?;

            let contract_id = hex::encode(contract_id);

            println!("   > Pay with {}", sender.preview());
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", hex::encode(init_args));
            println!("   > Deploy nonce = {}", deploy_nonce);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > Calculated Contract Id = {}", contract_id);
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        _ => Ok(true),
    }
}

fn status_emoji(status: bool) -> String {
    if status {
        "✅".to_string()
    } else {
        "❌".to_string()
    }
}

impl Display for ProfileSelect<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileSelect::Index(index, profile) => write!(
                f,
                "{}\n  {}\n  {}",
                Profile::index_string(*index),
                profile.shielded_account_preview(),
                profile.public_account_preview(),
            ),
            ProfileSelect::New => write!(f, "Create a new profile"),
            ProfileSelect::Exit => write!(f, "Exit"),
        }
    }
}

impl Display for MainMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MainMenu::Load(path) => {
                write!(f, "Load wallet from {}", path.wallet.display())
            }
            MainMenu::Create => write!(f, "Create a new wallet"),
            MainMenu::Recover => {
                write!(f, "Recover a lost wallet using recovery phrase")
            }
            MainMenu::Exit => write!(f, "Exit"),
        }
    }
}
