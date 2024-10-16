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
    Address, Error, Wallet, WalletPath, MAX_ADDRESSES,
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
        // let the user choose (or create) an address
        let addr_idx = match menu_addr(wallet)? {
            AddrSelect::AddressIndex(addr_idx) => addr_idx,
            AddrSelect::NewAddress => {
                if wallet.addresses().len() >= MAX_ADDRESSES {
                    println!(
                        "Cannot create more addresses, this wallet only supports up to {MAX_ADDRESSES} addresses"
                    );
                    std::process::exit(0);
                }

                let addr_idx = wallet.add_address();
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

                addr_idx
            }
            AddrSelect::Exit => std::process::exit(0),
        };

        let phoenix = Address::Phoenix {
            index: Some(addr_idx),
            addr: *wallet.phoenix_pk(addr_idx)?,
        };

        let moonlight = Address::Bls {
            index: Some(addr_idx),
            addr: *wallet.bls_pk(addr_idx)?,
        };
        loop {
            let is_synced = wallet.is_synced().await?;
            // get balance for this address
            prompt::hide_cursor()?;
            let moonlight_bal = wallet.get_moonlight_balance(addr_idx).await?;
            let phoenix_bal = wallet.get_phoenix_balance(addr_idx).await?;
            let phoenix_spendable = phoenix_bal.spendable.into();
            let phoenix_total: Dusk = phoenix_bal.value.into();

            prompt::hide_cursor()?;

            // display address information
            println!();
            println!();
            // display phoenix balance and keys information
            if is_synced {
                println!(
                    "{0: <23} - Spendable: {phoenix_spendable}",
                    "Phoenix Balance",
                );
                println!("{0: <23} - Total: {phoenix_total}", "",);
            } else {
                println!("Syncing Phoenix Balance",);
            }
            println!("Phoenix Address         - {phoenix}");
            println!();

            // display moonlight balance and keys information
            println!("{0: <23} - Total: {moonlight_bal}", "Moonlight Balance",);
            println!("Moonlight Address       - {moonlight}");
            println!();

            // request operation to perform
            let op = match wallet.is_online().await {
                true => menu_op(
                    wallet,
                    addr_idx,
                    phoenix_spendable,
                    moonlight_bal,
                    settings,
                    is_synced,
                ),
                false => menu_op_offline(addr_idx, settings),
            };

            // perform operations with this address
            match op? {
                AddrOp::Run(cmd) => {
                    // request confirmation before running
                    if confirm(&cmd)? {
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
                AddrOp::Back => break,
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum AddrSelect {
    AddressIndex(u8),
    NewAddress,
    Exit,
}

fn address_idx_string(addr_idx: u8) -> String {
    if addr_idx == 0 {
        "Default Address".to_string()
    } else {
        format!("Address no {:4}", addr_idx)
    }
}

/// Allows the user to choose an address from the selected wallet
/// to start performing operations.
fn menu_addr(wallet: &Wallet<WalletFile>) -> anyhow::Result<AddrSelect> {
    let mut address_menu = Menu::title("Addresses");
    let total_addresses = wallet.addresses().len() as u8;
    for addr_idx in 0..total_addresses {
        address_menu = address_menu.add(
            AddrSelect::AddressIndex(addr_idx),
            address_idx_string(addr_idx),
        );
    }

    let remaining_addresses =
        MAX_ADDRESSES.saturating_sub(total_addresses as usize);

    let mut action_menu = Menu::new();
    // only show the option to create a new address if we don't already have
    // `MAX_ADDRESSES`
    if remaining_addresses > 0 {
        action_menu = action_menu
            .separator()
            .add(AddrSelect::NewAddress, "New address")
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

    action_menu = action_menu.separator().add(AddrSelect::Exit, "Exit");

    let menu = address_menu.extend(action_menu);
    let questions = Question::select("theme")
        .message("Please select an address")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;
    Ok(menu.answer(&answer).to_owned())
}

/// Allows the user to choose an operation to perform with the selected
/// transaction type
fn transaction_op_menu_moonlight() -> anyhow::Result<AddrOp> {
    use TransactionOp::*;
    let menu = Menu::title("Moonlight Transaction Operations")
        .separator()
        .add(Back, "Back");

    let questions = Question::select("theme")
        .message("Please select an operation")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;

    let val = menu.answer(&answer).to_owned();

    let x = match val {
        History => AddrOp::Back,
        Back => AddrOp::Back,
    };

    Ok(x)
}

/// Allows the user to choose an operation to perform with the selected
/// transaction type
fn transaction_op_menu_phoenix(addr_idx: u8) -> anyhow::Result<AddrOp> {
    use TransactionOp::*;
    let menu = Menu::title("Phoenix Transaction Operations")
        .add(History, "Phoenix Transaction History")
        .separator()
        .add(Back, "Back");

    let questions = Question::select("theme")
        .message("Please select an operation")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;

    let val = menu.answer(&answer).to_owned();

    let x = match val {
        History => AddrOp::Run(Box::new(Command::PhoenixHistory {
            addr_idx: Some(addr_idx),
        })),
        Back => AddrOp::Back,
    };

    Ok(x)
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum AddrOp {
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
    // Withdraw
    Withdraw,
    // Contract Deploy
    ContractDeploy,
    // Contract Call
    ContractCall,
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
    History,
    Back,
}

/// Allows the user to choose the operation to perform for the
/// selected address
fn menu_op(
    wallet: &Wallet<WalletFile>,
    addr_idx: u8,
    phoenix_balance: Dusk,
    moonlight_balance: Dusk,
    settings: &Settings,
    is_synced: bool,
) -> anyhow::Result<AddrOp> {
    use CommandMenuItem as CMI;

    let mut cmd_menu = Menu::new()
        .add(CMI::Transfer, "Transfer")
        .add(CMI::Stake, "Stake")
        .add(CMI::Unstake, "Unstake")
        .add(CMI::Withdraw, "Withdraw Stake Reward")
        .add(CMI::StakeInfo, "Check Existing Stake")
        .add(CMI::ContractDeploy, "Contract Deploy")
        .add(CMI::ContractCall, "Contract Call")
        .add(CMI::PhoenixTransactions, "Phoenix Transactions")
        .add(CMI::MoonlightTransactions, "Moonlight Transactions")
        .add(CMI::PhoenixToMoonlight, "Convert Phoenix Dusk to Moonlight")
        .add(CMI::MoonlightToPhoenix, "Convert Moonlight Dusk to Phoenix")
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
                Address::Phoenix { .. } => {
                    (wallet.phoenix_address(addr_idx)?, phoenix_balance)
                }
                Address::Bls { .. } => {
                    (wallet.bls_address(addr_idx)?, moonlight_balance)
                }
            };
            AddrOp::Run(Box::new(Command::Transfer {
                sender: Some(sender),
                rcvr,
                amt: prompt::request_token_amt("transfer", balance)?,
                gas_limit: prompt::request_gas_limit(
                    gas::DEFAULT_LIMIT_TRANSFER,
                )?,
                memo: Some(prompt::request_str("memo")?),
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::Stake => {
            let (addr, balance) = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => {
                    (wallet.phoenix_address(addr_idx)?, phoenix_balance)
                }
                prompt::Protocol::Moonlight => {
                    (wallet.bls_address(addr_idx)?, moonlight_balance)
                }
            };
            AddrOp::Run(Box::new(Command::Stake {
                address: Some(addr),
                amt: prompt::request_stake_token_amt(balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::Unstake => {
            let addr = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => wallet.phoenix_address(addr_idx),
                prompt::Protocol::Moonlight => wallet.bls_address(addr_idx),
            }?;
            AddrOp::Run(Box::new(Command::Unstake {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }

        CMI::Withdraw => {
            let addr = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => wallet.phoenix_address(addr_idx),
                prompt::Protocol::Moonlight => wallet.bls_address(addr_idx),
            }?;
            AddrOp::Run(Box::new(Command::Withdraw {
                address: Some(addr),
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::ContractDeploy => {
            let addr = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => wallet.phoenix_address(addr_idx),
                prompt::Protocol::Moonlight => wallet.bls_address(addr_idx),
            }?;
            AddrOp::Run(Box::new(Command::ContractDeploy {
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
        CMI::ContractCall => {
            let addr = match prompt::request_protocol()? {
                prompt::Protocol::Phoenix => wallet.phoenix_address(addr_idx),
                prompt::Protocol::Moonlight => wallet.bls_address(addr_idx),
            }?;
            AddrOp::Run(Box::new(Command::ContractCall {
                address: Some(addr),
                contract_id: prompt::request_bytes("contract id")?,
                fn_name: prompt::request_str("function name to call")?,
                fn_args: prompt::request_bytes(
                    "arguments of calling function",
                )?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }

        CMI::PhoenixTransactions => transaction_op_menu_phoenix(addr_idx)?,
        CMI::MoonlightTransactions => transaction_op_menu_moonlight()?,
        CMI::StakeInfo => AddrOp::Run(Box::new(Command::StakeInfo {
            addr_idx: Some(addr_idx),
            reward: false,
        })),
        CMI::MoonlightToPhoenix => {
            AddrOp::Run(Box::new(Command::MoonlightToPhoenix {
                addr_idx: Some(addr_idx),
                amt: prompt::request_token_amt("convert", moonlight_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::PhoenixToMoonlight => {
            AddrOp::Run(Box::new(Command::PhoenixToMoonlight {
                addr_idx: Some(addr_idx),
                amt: prompt::request_token_amt("convert", phoenix_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT_CALL)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::CalculateContractId => {
            AddrOp::Run(Box::new(Command::CalculateContractId {
                addr_idx: Some(addr_idx),
                deploy_nonce: prompt::request_nonce()?,
                code: prompt::request_contract_code()?,
            }))
        }
        CMI::Export => AddrOp::Run(Box::new(Command::Export {
            addr_idx: Some(addr_idx),
            name: None,
            dir: prompt::request_dir("export keys", settings.profile.clone())?,
        })),
        CMI::Back => AddrOp::Back,
    };
    Ok(res)
}

/// Allows the user to choose the operation to perform for the
/// selected address while in offline mode
fn menu_op_offline(
    addr_idx: u8,
    settings: &Settings,
) -> anyhow::Result<AddrOp> {
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
        CMI::Export => AddrOp::Run(Box::new(Command::Export {
            addr_idx: Some(addr_idx),
            name: None,
            dir: prompt::request_dir("export keys", settings.profile.clone())?,
        })),
        CMI::Back => AddrOp::Back,
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
fn confirm(cmd: &Command) -> anyhow::Result<bool> {
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
            sender.same_protocol(rcvr)?;
            let max_fee = gas_limit * gas_price;
            println!("   > Sender = {sender}");
            println!("   > Recipient = {rcvr}");
            println!("   > Amount to transfer = {amt} DUSK");
            if let Some(memo) = memo {
                println!("   > Memo = {memo}");
            }
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Bls { .. } = sender {
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
            println!("   > Send from {sender}",);
            println!("   > Amount to stake = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Bls { .. } = sender {
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
            let max_fee = gas_limit * gas_price;
            println!("   > Send from {sender}",);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Bls { .. } = sender {
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
            println!("   > Send from {sender}",);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Bls { .. } = sender {
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

            println!("   > Deploy from = {sender}",);
            println!("   > Code len = {}", code_len);
            println!("   > Init args = {}", hex::encode(init_args));
            println!("   > Deploy nonce = {}", deploy_nonce);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            if let Address::Bls { .. } = sender {
                println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            }
            prompt::ask_confirm()
        }
        _ => Ok(true),
    }
}
