// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod command_menu;

use std::fmt::Display;

use bip39::{Language, Mnemonic, MnemonicType};
use inquire::{InquireError, Select};
use rusk_wallet::currency::Dusk;
use rusk_wallet::dat::{DatFileVersion, LATEST_VERSION};
use rusk_wallet::{Address, Error, Profile, Wallet, WalletPath, MAX_PROFILES};

use crate::io::{self, prompt};
use crate::settings::Settings;
use crate::{gen_salt, Command, GraphQL, RunResult, WalletFile};

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

            let op = if !wallet.is_online().await {
                println!("\r{}", profile.shielded_account_string());
                println!("{}", profile.public_account_string());
                println!();

                command_menu::offline(profile_index, settings)
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

                command_menu::online(
                    profile_index,
                    wallet,
                    phoenix_spendable,
                    moonlight_bal,
                    settings,
                )
                .await
            };

            prompt::hide_cursor()?;

            // perform operations with this profile
            match op {
                Ok(ProfileOp::Run(cmd)) => {
                    // request confirmation before running
                    if confirm(&cmd, wallet)? {
                        // run command
                        prompt::hide_cursor()?;
                        let res = cmd.run(wallet, settings).await?;

                        prompt::show_cursor()?;
                        // output results
                        println!("\r{}", res);
                        if let RunResult::Tx(hash) = res {
                            let tx_id = hex::encode(hash.to_bytes());

                            // Wait for transaction confirmation
                            // from network
                            let gql = GraphQL::new(
                                settings.state.to_string(),
                                io::status::interactive,
                            )?;
                            gql.wait_for(&tx_id).await?;

                            if let Some(explorer) = &settings.explorer {
                                let url = format!("{explorer}{tx_id}");
                                println!("> URL: {url}");
                                prompt::launch_explorer(url)?;
                            }
                        }
                    }
                }
                Ok(ProfileOp::Stay) => (),
                Ok(ProfileOp::Back) => {
                    break;
                }
                Err(e) => match e.downcast_ref::<InquireError>() {
                    Some(InquireError::OperationCanceled) => (),
                    _ => return Err(e),
                },
            };
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum ProfileSelect<'a> {
    Index(u8, &'a Profile),
    New,
    Back,
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
        ProfileSelect::Back => Err(InquireError::OperationCanceled.into()),
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

    menu_items.push(ProfileSelect::Back);

    let mut select = Select::new("Your Profiles", menu_items);

    // UNWRAP: Its okay to unwrap because the default help message
    // is provided by inquire Select struct
    let mut msg = Select::<ProfileSelect>::DEFAULT_HELP_MESSAGE
        .unwrap()
        .to_owned();

    if let Some(rx) = &wallet.state()?.sync_rx {
        if let Ok(status) = rx.try_recv() {
            msg = format!("Sync Status: {status}");
        } else {
            msg = "Waiting for Sync to complete..".to_string();
        }
    }

    select = select.with_help_message(&msg);

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
    file_version_and_salt: Result<(DatFileVersion, Option<[u8; 32]>), Error>,
) -> anyhow::Result<Wallet<WalletFile>> {
    let wallet_found =
        wallet_path.inner().exists().then(|| wallet_path.clone());

    let password = &settings.password;

    // display main menu
    let wallet = match menu_wallet(wallet_found, settings).await? {
        MainMenu::Load(path) => {
            let (file_version, salt) = file_version_and_salt?;
            let mut attempt = 1;
            loop {
                let pwd = prompt::request_auth(
                    "Please enter your wallet password",
                    password,
                    salt.as_ref(),
                    file_version,
                )?;
                match Wallet::from_file(WalletFile {
                    path: path.clone(),
                    pwd,
                    salt,
                }) {
                    Ok(wallet) => break wallet,
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
        MainMenu::Create => {
            // create a new randomly generated mnemonic phrase
            let mnemonic =
                Mnemonic::new(MnemonicType::Words12, Language::English);
            let salt = gen_salt();
            // ask user for a password to secure the wallet
            let pwd = prompt::create_password(
                password,
                Some(&salt),
                DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            )?;
            // display the mnemonic phrase
            prompt::confirm_mnemonic_phrase(&mnemonic)?;
            // create and store the wallet
            let mut w = Wallet::new(mnemonic)?;
            let path = wallet_path.clone();
            w.save_to(WalletFile {
                path,
                pwd,
                salt: Some(salt),
            })?;
            w
        }
        MainMenu::Recover => {
            // ask user for 12-word mnemonic phrase
            let phrase = prompt::request_mnemonic_phrase()?;
            let salt = gen_salt();
            // ask user for a password to secure the wallet, create the latest
            // wallet file from the seed
            let pwd = prompt::create_password(
                &None,
                Some(&salt),
                DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            )?;
            // create and store the recovered wallet
            let mut w = Wallet::new(phrase)?;
            let path = wallet_path.clone();
            w.save_to(WalletFile {
                path,
                pwd,
                salt: Some(salt),
            })?;
            w
        }
        MainMenu::Exit => std::process::exit(0),
    };

    Ok(wallet)
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
    wallet_found: Option<WalletPath>,
    settings: &Settings,
) -> anyhow::Result<MainMenu> {
    // create the wallet menu
    let mut menu_items = Vec::new();

    if let Some(wallet_path) = wallet_found {
        menu_items.push(MainMenu::Load(wallet_path));
        menu_items.push(MainMenu::Create);
        menu_items.push(MainMenu::Recover);
    } else {
        menu_items.push(MainMenu::Create);
        menu_items.push(MainMenu::Recover);
    }

    menu_items.push(MainMenu::Exit);

    let emoji_state = status_emoji(settings.check_state_con().await.is_ok());
    let emoji_prover = status_emoji(settings.check_prover_con().await.is_ok());

    let state_status = format!("{} State: {}", emoji_state, settings.state);
    let prover_status = format!("{} Prover: {}", emoji_prover, settings.prover);

    let menu = format!(
        "Welcome\n   {state_status}\n   {prover_status}   \nWhat would you like to do?",
    );

    // let the user choose an option
    let select = Select::new(menu.as_str(), menu_items);

    Ok(select.prompt()?)
}

/// Request user confirmation for a transfer transaction
fn confirm(cmd: &Command, wallet: &Wallet<WalletFile>) -> anyhow::Result<bool> {
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

        Command::Withdraw {
            address,
            gas_limit,
            gas_price,
        } => {
            let sender = address.as_ref().ok_or(Error::BadAddress)?;
            let max_fee = gas_limit * gas_price;
            let withdraw_from =
                wallet.public_address(wallet.find_index(sender)?)?;

            println!("   > Pay with {}", sender.preview());
            println!("   > Withdraw rewards from {}", withdraw_from.preview());
            println!("   > Receive rewards at {}", sender.preview());
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
                code_bytes,
                *deploy_nonce,
            )?;

            let contract_id = hex::encode(contract_id);

            println!("   > Pay with {}", sender.preview());
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", init_args);
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

impl<'a> Display for ProfileSelect<'a> {
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
            ProfileSelect::Back => write!(f, "Back"),
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
