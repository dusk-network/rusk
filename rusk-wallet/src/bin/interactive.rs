// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod command_menu;

use bip39::{Language, Mnemonic, MnemonicType};
use requestty::Question;
use rusk_wallet::{
    currency::Dusk,
    dat::{DatFileVersion, LATEST_VERSION},
    Address, Error, Profile, Wallet, WalletPath, MAX_PROFILES,
};

use crate::{
    io::{self, prompt},
    settings::Settings,
    Command, GraphQL, Menu, RunResult, WalletFile,
};

/// Run the interactive UX loop with a loaded wallet
pub(crate) async fn run_loop(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
) -> anyhow::Result<()> {
    loop {
        // let the user choose (or create) a profile
        let profile_idx = match menu_profile(wallet)? {
            ProfileSelect::Index(profile_idx) => profile_idx,
            ProfileSelect::New => {
                if wallet.profiles().len() >= MAX_PROFILES {
                    println!(
                        "Cannot create more profiles, this wallet only supports up to {MAX_PROFILES} profiles"
                    );
                    std::process::exit(0);
                }

                let profile_idx = wallet.add_profile();
                let file_version = wallet.get_file_version()?;

                let password = &settings.password;
                // if the version file is old, ask for password and save as
                // latest dat file
                if file_version.is_old() {
                    let pwd = prompt::request_auth(
                        "Updating your wallet data file, please enter your wallet password ",
                        password,
                        DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
                    )?;

                    wallet.save_to(WalletFile {
                        path: wallet.file().clone().unwrap().path,
                        pwd,
                    })?;
                } else {
                    // else just save
                    wallet.save()?;
                }

                profile_idx
            }
            ProfileSelect::Exit => std::process::exit(0),
        };

        loop {
            let profile = &wallet.profiles()[profile_idx as usize];
            prompt::hide_cursor()?;

            let op = if !wallet.is_online().await {
                println!("{}", profile.shielded_address_string());
                println!("{}", profile.public_account_string());
                println!();

                command_menu::offline(profile_idx, settings)
            } else {
                // get balance for this profile
                let moonlight_bal =
                    wallet.get_moonlight_balance(profile_idx).await?;
                let phoenix_bal =
                    wallet.get_phoenix_balance(profile_idx).await?;
                let phoenix_spendable = phoenix_bal.spendable.into();
                let phoenix_total: Dusk = phoenix_bal.value.into();

                // display profile information
                // display shielded balance and keys information
                println!("{}", profile.shielded_address_string());
                let is_synced = wallet.is_synced().await?;
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
                    profile_idx,
                    wallet,
                    phoenix_spendable,
                    moonlight_bal,
                    settings,
                )
            };

            prompt::hide_cursor()?;

            // perform operations with this profile
            match op? {
                ProfileOp::Run(cmd) => {
                    // request confirmation before running
                    if confirm(&cmd, wallet)? {
                        // run command
                        prompt::hide_cursor()?;
                        let result = cmd.run(wallet, settings).await;
                        prompt::show_cursor()?;
                        // output results
                        match result {
                            Ok(res) => {
                                println!("\r{}", res);
                                if let RunResult::Tx(hash) = res {
                                    let tx_id = hex::encode(hash.to_bytes());

                                    // Wait for transaction confirmation from
                                    // network
                                    let gql = GraphQL::new(
                                        settings.state.to_string(),
                                        io::status::interactive,
                                    );
                                    gql.wait_for(&tx_id).await?;

                                    if let Some(explorer) = &settings.explorer {
                                        let url = format!("{explorer}{tx_id}");
                                        println!("> URL: {url}");
                                        prompt::launch_explorer(url)?;
                                    }
                                }
                            }

                            Err(err) => println!("{err}"),
                        }
                    }
                }
                ProfileOp::Back => break,
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum ProfileSelect {
    Index(u8),
    New,
    Exit,
}

/// Allows the user to choose an profile from the selected wallet
/// to start performing operations.
fn menu_profile(wallet: &Wallet<WalletFile>) -> anyhow::Result<ProfileSelect> {
    let mut profile_menu = Menu::title("Profiles");
    for (profile_idx, profile) in wallet.profiles().iter().enumerate() {
        let profile_idx = profile_idx as u8;
        let profile_str = format!(
            "{}\n  {}\n  {}",
            Profile::index_string(profile_idx),
            profile.shielded_address_preview(),
            profile.public_account_preview(),
        );
        profile_menu =
            profile_menu.add(ProfileSelect::Index(profile_idx), profile_str);
    }

    let remaining_profiles =
        MAX_PROFILES.saturating_sub(wallet.profiles().len());

    let mut action_menu = Menu::new();
    // only show the option to create a new profile if we don't already have
    // `MAX_PROFILES`
    if remaining_profiles > 0 {
        action_menu = action_menu
            .separator()
            .add(ProfileSelect::New, "New profile")
    };

    if let Some(rx) = &wallet.state()?.sync_rx {
        if let Ok(status) = rx.try_recv() {
            action_menu = action_menu
                .separator()
                .separator_msg(format!("Sync Status: {status}"));
        } else {
            action_menu = action_menu
                .separator()
                .separator_msg("Waiting for Sync to complete..");
        }
    }

    action_menu = action_menu.separator().add(ProfileSelect::Exit, "Exit");

    let menu = profile_menu.extend(action_menu);
    let questions = Question::select("theme")
        .message("Please select a profile")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;
    Ok(menu.answer(&answer).to_owned())
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum ProfileOp {
    Run(Box<Command>),
    Back,
}

/// Allows the user to load a wallet interactively
pub(crate) fn load_wallet(
    wallet_path: &WalletPath,
    settings: &Settings,
    file_version: Result<DatFileVersion, Error>,
) -> anyhow::Result<Wallet<WalletFile>> {
    let wallet_found =
        wallet_path.inner().exists().then(|| wallet_path.clone());

    let password = &settings.password;

    // display main menu
    let wallet = match menu_wallet(wallet_found)? {
        MainMenu::Load(path) => {
            let file_version = file_version?;
            let mut attempt = 1;
            loop {
                let pwd = prompt::request_auth(
                    "Please enter your wallet password",
                    password,
                    file_version,
                )?;
                match Wallet::from_file(WalletFile {
                    path: path.clone(),
                    pwd,
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
            // ask user for a password to secure the wallet
            let pwd = prompt::create_password(
                password,
                DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            )?;
            // display the recovery phrase
            prompt::confirm_recovery_phrase(&mnemonic)?;
            // create and store the wallet
            let mut w = Wallet::new(mnemonic)?;
            let path = wallet_path.clone();
            w.save_to(WalletFile { path, pwd })?;
            w
        }
        MainMenu::Recover => {
            // ask user for 12-word recovery phrase
            let phrase = prompt::request_recovery_phrase()?;
            // ask user for a password to secure the wallet, create the latest
            // wallet file from the seed
            let pwd = prompt::create_password(
                &None,
                DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
            )?;
            // create and store the recovered wallet
            let mut w = Wallet::new(phrase)?;
            let path = wallet_path.clone();
            w.save_to(WalletFile { path, pwd })?;
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
fn menu_wallet(wallet_found: Option<WalletPath>) -> anyhow::Result<MainMenu> {
    // create the wallet menu
    let mut menu = Menu::new();

    if let Some(wallet_path) = wallet_found {
        menu = menu
            .separator()
            .add(MainMenu::Load(wallet_path), "Access your wallet")
            .separator()
            .add(MainMenu::Create, "Replace your wallet with a new one")
            .add(
                MainMenu::Recover,
                "Replace your wallet with a lost one using the recovery phrase",
            )
    } else {
        menu = menu.add(MainMenu::Create, "Create a new wallet").add(
            MainMenu::Recover,
            "Access a lost wallet using the recovery phrase",
        )
    }

    // create the action menu
    menu = menu.separator().add(MainMenu::Exit, "Exit");

    // let the user choose an option
    let questions = Question::select("theme")
        .message("What would you like to do?")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;
    Ok(menu.answer(&answer).to_owned())
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
            let sender = sender.as_ref().expect("sender to be a valid address");
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
            amt,
            gas_limit,
            gas_price,
        } => {
            let sender =
                address.as_ref().expect("address to be a valid address");
            let max_fee = gas_limit * gas_price;
            let stake_to = wallet.public_address(wallet.find_index(sender)?)?;
            println!("   > Pay with {}", sender.preview());
            println!("   > Stake to {}", stake_to.preview());
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
            let sender =
                address.as_ref().expect("address to be a valid address");
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
            let sender =
                address.as_ref().expect("address to be a valid address");
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
            let sender =
                address.as_ref().expect("address to be a valid address");
            let code_len = code.metadata()?.len();
            let max_fee = gas_limit * gas_price;

            println!("   > Pay with {}", sender.preview());
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", hex::encode(init_args));
            println!("   > Deploy nonce = {}", deploy_nonce);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public(_) = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        _ => Ok(true),
    }
}
