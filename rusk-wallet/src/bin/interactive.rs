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
    gas, Address, Error, Wallet, WalletPath, MAX_ADDRESSES,
};

use crate::command::DEFAULT_STAKE_GAS_LIMIT;
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
        let addr = match menu_addr(wallet)? {
            AddrSelect::Address(addr) => *addr,
            AddrSelect::NewAddress => {
                if wallet.addresses().len() >= MAX_ADDRESSES {
                    println!(
                        "Cannot create more addresses, this wallet only supports up to {MAX_ADDRESSES} addresses"
                    );
                    std::process::exit(0);
                }

                let addr = wallet.new_address().clone();
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

                addr
            }
            AddrSelect::Exit => std::process::exit(0),
        };

        let index = addr.index()?;

        let moonlight = Address::Bls {
            index: Some(index),
            addr: wallet.bls_public_key(addr.index()?),
        };

        let is_synced = wallet.is_synced().await?;

        loop {
            // get balance for this address
            prompt::hide_cursor()?;
            let phoenix_bal = wallet.get_phoenix_balance(&addr).await?;
            let moonlight_bal =
                wallet.get_moonlight_balance(&moonlight).await?;
            let spendable = phoenix_bal.spendable.into();
            let total: Dusk = phoenix_bal.value.into();

            let mut spendable_str = format!("{}", spendable);
            let mut total_str = format!("{}", total);
            let mut moonlight_bal_str = format!("{}", moonlight_bal);

            // display placeholders if not synced yet
            if !is_synced {
                moonlight_bal_str = String::from("syncing...");
                spendable_str = moonlight_bal_str.clone();
                total_str = moonlight_bal_str.clone();
            }

            prompt::hide_cursor()?;

            // display address information
            println!();
            println!();
            println!(
                "{0: <20} - Total: {moonlight_bal_str}",
                "Moonlight Balance"
            );
            println!("{0: <20} {moonlight}", "Moonlight Address");

            println!();
            println!(
                "{0: <20} - Spendable: {spendable_str}",
                "Phoenix Balance",
            );
            println!("{0: <20} - Total: {total_str}", "");
            println!("{0: <20} {addr}", "Phoenix Address");
            println!();

            // request operation to perform
            let op = match wallet.is_online().await {
                true => menu_op(
                    addr.clone(),
                    spendable,
                    moonlight_bal,
                    settings,
                    is_synced,
                ),
                false => menu_op_offline(addr.clone(), settings),
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
    Address(Box<Address>),
    NewAddress,
    Exit,
}

/// Allows the user to choose an address from the selected wallet
/// to start performing operations.
fn menu_addr(wallet: &Wallet<WalletFile>) -> anyhow::Result<AddrSelect> {
    let mut address_menu = Menu::title("Addresses");
    for addr in wallet.addresses() {
        let preview = addr.preview();
        address_menu = address_menu
            .add(AddrSelect::Address(Box::new(addr.clone())), preview);
    }

    let remaining_addresses =
        MAX_ADDRESSES.saturating_sub(wallet.addresses().len());
    let mut action_menu = Menu::new()
        .separator()
        .add(AddrSelect::NewAddress, "New address");

    // show warning if less than
    if remaining_addresses < 5 {
        action_menu = action_menu.separator().separator_msg(format!(
            "\x1b[93m{}\x1b[0m This wallet only supports up to {MAX_ADDRESSES} addresses, you have {} addresses ",
            "Warning:",
            wallet.addresses().len()
        ));
    }

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
fn transaction_op_menu_moonlight(
    addr: Address,
    moonlight_bal: Dusk,
) -> anyhow::Result<AddrOp> {
    use TransactionOp::*;
    let menu = Menu::title("Moonlight Transaction Operations")
        .add(Transfer, "Moonlight Transfer")
        .add(Memo, "Moonlight Transfer with Memo")
        .add(Stake, "Moonlight Stake")
        .add(Unstake, "Moonlight Unstake")
        .add(Withdraw, "Moonlight Withdraw")
        .add(ContractDeploy, "Moonlight Contract Deploy")
        .add(ContractCall, "Moonlight Contract call")
        //.add(History, "Moonlight Transaction History")
        .separator()
        .add(Back, "Back");

    let questions = Question::select("theme")
        .message("Please select an operation")
        .choices(menu.clone())
        .build();

    let answer = requestty::prompt_one(questions)?;

    let val = menu.answer(&answer).to_owned();

    let x = match val {
        Transfer => AddrOp::Run(Box::new(Command::MoonlightTransfer {
            sndr: Some(addr),
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_token_amt("transfer", moonlight_bal)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Memo => AddrOp::Run(Box::new(Command::MoonlightMemo {
            sndr: Some(addr),
            memo: prompt::request_str("memo")?,
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_optional_token_amt("transfer", moonlight_bal)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Stake => AddrOp::Run(Box::new(Command::MoonlightStake {
            addr: Some(addr),
            amt: prompt::request_token_amt("stake", moonlight_bal)?,
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Unstake => AddrOp::Run(Box::new(Command::MoonlightUnstake {
            addr: Some(addr),
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Withdraw => AddrOp::Run(Box::new(Command::MoonlightWithdraw {
            addr: Some(addr),
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        ContractDeploy => {
            AddrOp::Run(Box::new(Command::MoonlightContractDeploy {
                addr: Some(addr),
                code: prompt::request_contract_code()?,
                init_args: prompt::request_bytes("init arguments")?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        ContractCall => AddrOp::Run(Box::new(Command::MoonlightContractCall {
            addr: Some(addr),
            contract_id: prompt::request_bytes("contract id")?,
            fn_name: prompt::request_str("function name to call")?,
            fn_args: prompt::request_bytes("arguments of calling function")?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        History => AddrOp::Back,
        Back => AddrOp::Back,
    };

    Ok(x)
}

/// Allows the user to choose an operation to perform with the selected
/// transaction type
fn transaction_op_menu_phoenix(
    addr: Address,
    phoenix_balance: Dusk,
) -> anyhow::Result<AddrOp> {
    use TransactionOp::*;
    let menu = Menu::title("Phoenix Transaction Operations")
        .add(Transfer, "Phoenix Transfer")
        .add(Memo, "Phoenix Transfer with Memo")
        .add(Stake, "Phoenix Stake")
        .add(Unstake, "Phoenix Unstake")
        .add(Withdraw, "Phoenix Withdraw")
        .add(ContractDeploy, "Phoenix Contract Deploy")
        .add(ContractCall, "Phoenix Contract call")
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
        Transfer => AddrOp::Run(Box::new(Command::PhoenixTransfer {
            sndr: Some(addr),
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_token_amt("transfer", phoenix_balance)?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Memo => AddrOp::Run(Box::new(Command::PhoenixMemo {
            sndr: Some(addr),
            memo: prompt::request_str("memo")?,
            rcvr: prompt::request_rcvr_addr("recipient")?,
            amt: prompt::request_optional_token_amt(
                "transfer",
                phoenix_balance,
            )?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Stake => AddrOp::Run(Box::new(Command::PhoenixStake {
            addr: Some(addr),
            amt: prompt::request_token_amt("stake", phoenix_balance)?,
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Unstake => AddrOp::Run(Box::new(Command::PhoenixUnstake {
            addr: Some(addr),
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        Withdraw => AddrOp::Run(Box::new(Command::PhoenixWithdraw {
            addr: Some(addr),
            gas_limit: prompt::request_gas_limit(DEFAULT_STAKE_GAS_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        ContractDeploy => {
            AddrOp::Run(Box::new(Command::PhoenixContractDeploy {
                addr: Some(addr),
                code: prompt::request_contract_code()?,
                init_args: prompt::request_bytes("init arguments")?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        ContractCall => AddrOp::Run(Box::new(Command::PhoenixContractCall {
            addr: Some(addr),
            contract_id: prompt::request_bytes("contract id")?,
            fn_name: prompt::request_str("function name to call")?,
            fn_args: prompt::request_bytes("arguments of calling function")?,
            gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
            gas_price: prompt::request_gas_price()?,
        })),
        History => {
            AddrOp::Run(Box::new(Command::PhoenixHistory { addr: Some(addr) }))
        }
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
    // Phoenix
    PhoenixTransactions,
    // Moonlight
    MoonlightTransactions,
    // Conversion
    PhoenixToMoonlight,
    MoonlightToPhoenix,
    // Others
    StakeInfo,
    Export,
    Back,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum TransactionOp {
    Transfer,
    Memo,
    Stake,
    Unstake,
    Withdraw,
    ContractDeploy,
    ContractCall,
    // nor a deployment or a call
    History,
    Back,
}

/// Allows the user to choose the operation to perform for the
/// selected address
fn menu_op(
    addr: Address,
    phoenix_balance: Dusk,
    moonlight_balance: Dusk,
    settings: &Settings,
    is_synced: bool,
) -> anyhow::Result<AddrOp> {
    use CommandMenuItem as CMI;

    let mut cmd_menu = Menu::new()
        .add(CMI::StakeInfo, "Check Existing Stake")
        .add(CMI::PhoenixTransactions, "Phoenix Transactions")
        .add(CMI::MoonlightTransactions, "Moonlight Transactions")
        .add(CMI::PhoenixToMoonlight, "Convert Phoenix Dusk to Moonlight")
        .add(CMI::MoonlightToPhoenix, "Convert Moonlight Dusk to Phoenix")
        .add(CMI::Export, "Export provisioner key-pair")
        .separator()
        .add(CMI::Back, "Back")
        .separator();

    let msg = if !is_synced {
        cmd_menu = Menu::new()
            .separator()
            .add(CMI::StakeInfo, "Check Existing Stake")
            .add(CMI::Export, "Export provisioner key-pair")
            .separator()
            .add(CMI::Back, "Back")
            .separator();

        "Not Synced yet, Come back after its done."
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
        CMI::PhoenixTransactions => {
            transaction_op_menu_phoenix(addr, phoenix_balance)?
        }
        CMI::MoonlightTransactions => {
            transaction_op_menu_moonlight(addr, moonlight_balance)?
        }
        CMI::StakeInfo => AddrOp::Run(Box::new(Command::StakeInfo {
            addr: Some(addr),
            reward: false,
        })),
        CMI::MoonlightToPhoenix => {
            AddrOp::Run(Box::new(Command::MoonlightToPhoenix {
                addr: Some(addr),
                amt: prompt::request_token_amt("convert", moonlight_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::PhoenixToMoonlight => {
            AddrOp::Run(Box::new(Command::PhoenixToMoonlight {
                addr: Some(addr),
                amt: prompt::request_token_amt("convert", phoenix_balance)?,
                gas_limit: prompt::request_gas_limit(gas::DEFAULT_LIMIT)?,
                gas_price: prompt::request_gas_price()?,
            }))
        }
        CMI::Export => AddrOp::Run(Box::new(Command::Export {
            addr: Some(addr),
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
    addr: Address,
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
            addr: Some(addr),
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
        Command::PhoenixTransfer {
            sndr,
            rcvr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let sndr = sndr.as_ref().expect("sender to be a valid address");
            let max_fee = gas_limit * gas_price;
            println!("   > Send from = {}", sndr.preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightTransfer {
            sndr,
            rcvr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let sndr = sndr.as_ref().expect("sender to be a valid address");
            let max_fee = gas_limit * gas_price;
            println!("   > Send from = {}", sndr.preview());
            println!("   > Recipient = {}", rcvr.preview());
            println!("   > Amount to transfer = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::MoonlightStake {
            addr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Stake from {}", addr.preview());
            println!("   > Amount to stake = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::PhoenixStake {
            addr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Stake from {}", addr.preview());
            println!("   > Amount to stake = {} DUSK", amt);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::PhoenixUnstake {
            addr,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Unstake from {}", addr.preview());
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightUnstake {
            addr,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Unstake from {}", addr.preview());
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::PhoenixWithdraw {
            addr,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Reward from {}", addr.preview());
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        Command::MoonlightWithdraw {
            addr,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            println!("   > Reward from {}", addr.preview());
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::MoonlightContractDeploy {
            addr,
            code,
            init_args,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            let file_len = code.metadata()?.len();
            println!("   > Deploy contract from {}", addr.preview());
            println!("   > Wasm file length = {}", file_len);
            println!("   > Init args = {:?}", init_args);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            println!("   > ALERT: THIS IS A PUBLIC TRANSACTION");
            prompt::ask_confirm()
        }
        Command::PhoenixContractDeploy {
            addr,
            code,
            init_args,
            gas_limit,
            gas_price,
        } => {
            let addr = addr.as_ref().expect("address to be valid");
            let max_fee = gas_limit * gas_price;
            let file_len = code.metadata()?.len();
            println!("   > Deploy contract from {}", addr.preview());
            println!("   > Wasm file length = {}", file_len);
            println!("   > Init args = {:?}", init_args);
            println!("   > Max fee = {} DUSK", Dusk::from(max_fee));
            prompt::ask_confirm()
        }
        _ => Ok(true),
    }
}
