// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bip39::{Language, Mnemonic, MnemonicType};
use requestty::Question;
use rusk_wallet::{
    currency::Dusk,
    dat::{DatFileVersion, LATEST_VERSION},
    gas::{self},
    Address, Error, Profile, Wallet, WalletPath, MAX_PROFILES,
};

use crate::io;
use crate::io::prompt::request_auth;
use crate::io::GraphQL;
use crate::prompt;
use crate::settings::Settings;
use crate::Menu;
use crate::WalletFile;
use crate::{Command, RunResult};

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
                    let pwd = request_auth(
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
            let is_synced = wallet.is_synced().await?;
            // get balance for this profile
            prompt::hide_cursor()?;
            let moonlight_bal =
                wallet.get_moonlight_balance(profile_idx).await?;
            let phoenix_bal = wallet.get_phoenix_balance(profile_idx).await?;
            let phoenix_spendable = phoenix_bal.spendable.into();
            let phoenix_total: Dusk = phoenix_bal.value.into();

            prompt::hide_cursor()?;

            let profile = &wallet.profiles()[profile_idx as usize];

            // display profile information
            println!();
            println!();
            // display shielded balance and keys information
            println!("{}\n", profile.shielded_address_string());
            if is_synced {
                println!(
                    "{0: <15} - Spendable: {phoenix_spendable}",
                    "Shielded Balance",
                );
                println!("{0: <16} - Total: {phoenix_total}", "",);
            }

            // display public balance and keys information
            println!("{}\n", profile.public_account_string());
            if is_synced {
                println!("{0: <16} - Total: {moonlight_bal}", "Public Balance",);
            }

            // request operation to perform
            let op = match wallet.is_online().await {
                true => menu_op(
                    profile_idx,
                    wallet,
                    phoenix_spendable,
                    moonlight_bal,
                    settings,
                    is_synced,
                ),
                false => menu_op_offline(profile_idx, settings),
            };

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
                .separator_msg(format!("Sync Status: {}", status));
        } else {
            action_menu = action_menu
                .separator()
                .separator_msg("Waiting for Sync to complete..".to_string());
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

/// Allows the user to choose an operation to perform with the selected
/// transaction type
fn transaction_op_menu_moonlight(
    profile_idx: u8,
    moonlight_bal: Dusk,
) -> anyhow::Result<ProfileOp> {
    use TransactionOp::*;
    let menu = Menu::title("Public Transaction Operations")
        .add(Memo, "Public Transfer with Memo")
        .add(Withdraw, "Public Withdraw Stake Reward")
        .add(ContractDeploy, "Public Contract Deploy")
        .add(ContractCall, "Public Contract Call")
        //.add(History, "Public Transaction History")
        .separator()
        .add(Back, "Back");

    let questions = Question::select("theme")
        .message("Please select an operation")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;

    let val = menu.answer(&answer).to_owned();

    let x = match val {
        Memo => ProfileOp::Run(Box::new(Command::MoonlightMemo {
            profile_idx: Some(profile_idx),
            memo: prompt::request_str("memo")?,
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_optional_token_amt("transfer", moonlight_bal)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_TRANSFER)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Withdraw => ProfileOp::Run(Box::new(Command::MoonlightWithdraw {
            profile_idx: Some(profile_idx),
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
            gas_price: prompt::request_gas_price()?,
        })),
        ContractDeploy => {
            ProfileOp::Run(Box::new(Command::MoonlightContractDeploy {
                profile_idx: Some(profile_idx),
                code: prompt::request_contract_code()?,
                init_args: prompt::request_bytes("init arguments")?,
                deploy_nonce: prompt::request_nonce()?,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_DEPLOYMENT,
                )?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        ContractCall => {
            ProfileOp::Run(Box::new(Command::MoonlightContractCall {
                profile_idx: Some(profile_idx),
                contract_id: prompt::request_bytes("contract id")?,
                fn_name: prompt::request_str("function name to call")?,
                fn_args: prompt::request_bytes(
                    "arguments of calling function",
                )?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        History => ProfileOp::Back,
        Back => ProfileOp::Back,
    };

    Ok(x)
}

/// Allows the user to choose an operation to perform with the selected
/// transaction type
fn transaction_op_menu_phoenix(
    profile_idx: u8,
    phoenix_balance: Dusk,
) -> anyhow::Result<ProfileOp> {
    use TransactionOp::*;
    let menu = Menu::title("Shielded Transaction Operations")
        .add(Memo, "Shielded Transfer with Memo")
        .add(Withdraw, "Shielded Withdraw Stake Reward")
        .add(ContractDeploy, "Shielded Contract Deploy")
        .add(ContractCall, "Shielded Contract Call")
        .add(History, "Shielded Transaction History")
        .separator()
        .add(Back, "Back");

    let questions = Question::select("theme")
        .message("Please select an operation")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;

    let val = menu.answer(&answer).to_owned();

    let x = match val {
        Memo => ProfileOp::Run(Box::new(Command::PhoenixMemo {
            profile_idx: Some(profile_idx),
            memo: prompt::request_str("memo")?,
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_optional_token_amt(
                "transfer",
                phoenix_balance,
            )?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_TRANSFER)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Withdraw => ProfileOp::Run(Box::new(Command::PhoenixWithdraw {
            profile_idx: Some(profile_idx),
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
            gas_price: prompt::request_gas_price()?,
        })),
        ContractDeploy => {
            ProfileOp::Run(Box::new(Command::PhoenixContractDeploy {
                profile_idx: Some(profile_idx),
                code: prompt::request_contract_code()?,
                init_args: prompt::request_bytes("init arguments")?,
                deploy_nonce: prompt::request_nonce()?,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_DEPLOYMENT,
                )?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        ContractCall => {
            ProfileOp::Run(Box::new(Command::PhoenixContractCall {
                profile_idx: Some(profile_idx),
                contract_id: prompt::request_bytes("contract id")?,
                fn_name: prompt::request_str("function name to call")?,
                fn_args: prompt::request_bytes(
                    "arguments of calling function",
                )?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        History => ProfileOp::Run(Box::new(Command::PhoenixHistory {
            profile_idx: Some(profile_idx),
        })),
        Back => ProfileOp::Back,
    };

    Ok(x)
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum ProfileOp {
    Run(Box<Command>),
    Back,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum CommandMenuItem {
    // Transfer
    Transfer,
    // Stake
    Stake,
    // Unstake
    Unstake,
    // Phoenix
    PhoenixTransactions,
    // Moonlight
    MoonlightTransactions,
    // Conversion
    PhoenixToMoonlight,
    MoonlightToPhoenix,
    // Generate Contract ID.
    CalculateContractId,
    // Others
    StakeInfo,
    Export,
    Back,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum TransactionOp {
    Memo,
    Withdraw,
    ContractDeploy,
    ContractCall,
    // nor a deployment or a call
    History,
    Back,
}

/// Allows the user to choose the operation to perform for the
/// selected profile
fn menu_op(
    profile_idx: u8,
    wallet: &Wallet<WalletFile>,
    phoenix_balance: Dusk,
    moonlight_balance: Dusk,
    settings: &Settings,
    is_synced: bool,
) -> anyhow::Result<ProfileOp> {
    use CommandMenuItem as CMI;

    let mut cmd_menu = Menu::new()
        .add(CMI::Transfer, "Transfer")
        .add(CMI::Stake, "Stake")
        .add(CMI::Unstake, "Unstake")
        .add(CMI::StakeInfo, "Check Existing Stake")
        .add(CMI::PhoenixTransactions, "Shielded Transactions")
        .add(CMI::MoonlightTransactions, "Public Transactions")
        .add(
            CMI::PhoenixToMoonlight,
            "Convert shielded Dusk to public Dusk",
        )
        .add(
            CMI::MoonlightToPhoenix,
            "Convert public Dusk to shielded Dusk",
        )
        .add(CMI::CalculateContractId, "Calculate Contract ID")
        .add(CMI::Export, "Export provisioner key-pair")
        .separator()
        .add(CMI::Back, "Back")
        .separator();

    let msg = if !is_synced {
        cmd_menu = Menu::new()
            .add(CMI::StakeInfo, "Check Existing Stake")
            .add(CMI::Export, "Export provisioner key-pair")
            .separator()
            .add(CMI::Back, "Back")
            .separator();

        "The wallet is still syncing. Please come back to display new information."
    } else {
        "What do you like to do?"
    };

    let q = Question::select("theme")
        .message(msg)
        .choices(cmd_menu.clone())
        .build();

    let answer = requestty::prompt_one(q)?;
    let cmd = cmd_menu.answer(&answer).to_owned();

    let res = match cmd {
        CMI::Transfer => {
            let rcvr = prompt::request_rcvr_addr("recipient")?;
            let (sender, balance) = match &rcvr {
                Address::Shielded { .. } => {
                    (wallet.shielded_address(profile_idx)?, phoenix_balance)
                }
                Address::Public { .. } => {
                    (wallet.public_address(profile_idx)?, moonlight_balance)
                }
            };
            ProfileOp::Run(Box::new(Command::Transfer {
                sender: Some(sender),
                rcvr,
                amt: prompt::request_token_amt("transfer", balance)?,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_TRANSFER,
                )?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::Stake => {
            let (addr, balance) = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => {
                    (wallet.shielded_address(profile_idx)?, phoenix_balance)
                }
                prompt::Protocol::Moonlight => {
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
        CMI::Unstake => {
            let addr = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => {
                    wallet.shielded_address(profile_idx)
                }
                prompt::Protocol::Moonlight => {
                    wallet.public_address(profile_idx)
                }
            }?;
            ProfileOp::Run(Box::new(Command::Unstake {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::PhoenixTransactions => {
            transaction_op_menu_phoenix(profile_idx, phoenix_balance)?
        }
        CMI::MoonlightTransactions => {
            transaction_op_menu_moonlight(profile_idx, moonlight_balance)?
        }
        CMI::StakeInfo => ProfileOp::Run(Box::new(Command::StakeInfo {
            profile_idx: Some(profile_idx),
            reward: false,
        })),
        CMI::MoonlightToPhoenix => {
            ProfileOp::Run(Box::new(Command::MoonlightToPhoenix {
                profile_idx: Some(profile_idx),
                amt: prompt::request_token_amt("convert", moonlight_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::PhoenixToMoonlight => {
            ProfileOp::Run(Box::new(Command::PhoenixToMoonlight {
                profile_idx: Some(profile_idx),
                amt: prompt::request_token_amt("convert", phoenix_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::CalculateContractId => {
            ProfileOp::Run(Box::new(Command::CalculateContractId {
                profile_idx: Some(profile_idx),
                deploy_nonce: prompt::request_nonce()?,
                code: prompt::request_contract_code()?,
            }))
        }
        CMI::Export => ProfileOp::Run(Box::new(Command::Export {
            profile_idx: Some(profile_idx),
            name: None,
            dir: prompt::request_dir("export keys", settings.profile.clone())?,
        })),
        CMI::Back => ProfileOp::Back,
    };
    Ok(res)
}

/// Allows the user to choose the operation to perform for the
/// selected profile while in offline mode
fn menu_op_offline(
    profile_idx: u8,
    settings: &Settings,
) -> anyhow::Result<ProfileOp> {
    use CommandMenuItem as CMI;

    let cmd_menu = Menu::new()
        .separator()
        .add(CMI::Export, "Export provisioner key-pair")
        .separator()
        .add(CMI::Back, "Back");

    let q = Question::select("theme")
        .message("[OFFLINE] What would you like to do?")
        .choices(cmd_menu.clone())
        .build();

    let answer = requestty::prompt_one(q)?;
    let cmd = cmd_menu.answer(&answer).to_owned();

    let res = match cmd {
        CMI::Export => ProfileOp::Run(Box::new(Command::Export {
            profile_idx: Some(profile_idx),
            name: None,
            dir: prompt::request_dir("export keys", settings.profile.clone())?,
        })),
        CMI::Back => ProfileOp::Back,
        _ => unreachable!(),
    };
    Ok(res)
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
        } => {
            let sender = sender.as_ref().expect("sender to be a valid address");
            sender.same_protocol(rcvr)?;
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", sender.preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Public { .. } = sender {
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
            if let Address::Public { .. } = sender {
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
            if let Address::Public { .. } = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        Command::PhoenixWithdraw {
            profile_idx,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.shielded_address_preview());
            println!(
                "   > Withdraw rewards from {}",
                profile.staking_account_preview()
            );
            println!(
                "   > Receive rewards at {}",
                profile.shielded_address_preview()
            );
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightWithdraw {
            profile_idx,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.public_account_preview());
            println!(
                "   > Withdraw rewards from {}",
                profile.staking_account_preview()
            );
            println!(
                "   > Receive rewards at {}",
                profile.public_account_preview()
            );
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::PhoenixMemo {
            profile_idx,
            memo,
            rcvr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.shielded_address_preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            println!("   > Memo = {}", memo);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightMemo {
            profile_idx,
            memo,
            rcvr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.public_account_preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            println!("   > Memo = {}", memo);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::PhoenixContractDeploy {
            profile_idx,
            code,
            init_args,
            deploy_nonce,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let code_len = code.metadata()?.len();
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.shielded_address_preview());
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", hex::encode(init_args));
            println!("   > Deploy nonce = {}", deploy_nonce);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightContractDeploy {
            profile_idx,
            code,
            init_args,
            deploy_nonce,
            gas_limit,
            gas_price,
        } => {
            let index = check_index(profile_idx, wallet.profiles().len())?;
            let profile = &wallet.profiles()[index];
            let code_len = code.metadata()?.len();
            let max_fee = gas_limit * gas_price;
            println!("   > Pay with {}", profile.public_account_preview());
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", hex::encode(init_args));
            println!("   > Deploy nonce = {}", deploy_nonce);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        _ => Ok(true),
    }
}

fn check_index(
    index: &Option<u8>,
    total_profiles: usize,
) -> anyhow::Result<usize> {
    // if an index is given, check that it is smaller the wallet's profile-count
    if let Some(idx) = index {
        if total_profiles > u8::MAX.into() || *idx >= total_profiles as u8 {
            return Err(Error::Unauthorized.into());
        }
    }
    Ok(index.unwrap_or_default() as usize)
}
