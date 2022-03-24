// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    ExecutableCommand,
};

use blake3::Hash;
use requestty::Question;
use rusk_abi::dusk::*;

use super::store::LocalStore;
use crate::lib::crypto::MnemSeed;
use crate::lib::{
    Dusk, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE, MAX_CONVERTIBLE,
    MIN_CONVERTIBLE, MIN_GAS_LIMIT,
};
use crate::{CliCommand, Error};

/// Request the user to authenticate with a password
pub(crate) fn request_auth(msg: &str) -> Hash {
    let pwd = match env::var("RUSK_WALLET_PWD").ok() {
        Some(p) => p,

        None => {
            let q = Question::password("password")
                .message(format!("{}:", msg))
                .mask('*')
                .build();

            let a = requestty::prompt_one(q).expect("password");
            let p = a.as_string().unwrap();

            p.to_string()
        }
    };

    blake3::hash(pwd.as_bytes())
}

/// Request the user to create a wallet password
pub(crate) fn create_password() -> Hash {
    let pwd = match env::var("RUSK_WALLET_PWD") {
        Ok(p) => p,
        Err(_) => {
            let mut pwd = String::from("");

            let mut pwds_match = false;
            while !pwds_match {
                // enter password
                let q = Question::password("password")
                    .message("Enter a strong password for your wallet:")
                    .mask('*')
                    .build();
                let a = requestty::prompt_one(q).expect("password");
                let pwd1 = a.as_string().unwrap_or("").to_string();

                // confirm password
                let q = Question::password("password")
                    .message("Please confirm your password:")
                    .mask('*')
                    .build();
                let a =
                    requestty::prompt_one(q).expect("password confirmation");
                let pwd2 = a.as_string().unwrap_or("").to_string();

                // check match
                pwds_match = pwd1 == pwd2;
                if pwds_match {
                    pwd = pwd1.to_string()
                } else {
                    println!("Passwords don't match, please try again.");
                }
            }
            pwd
        }
    };

    let pwd = blake3::hash(pwd.as_bytes());
    pwd
}

/// Display the recovery phrase to the user and ask for confirmation
pub(crate) fn confirm_recovery_phrase(phrase: String) {
    // inform the user about the mnemonic phrase
    println!("The following phrase is essential for you to regain access to your wallet\nin case you lose access to this computer.");
    println!("Please print it or write it down and store it somewhere safe:");
    println!();
    println!("> {}", phrase);
    println!();

    // let the user confirm they have backed up their phrase
    loop {
        let q = requestty::Question::confirm("proceed")
            .message("Have you backed up your recovery phrase?")
            .build();

        let a = requestty::prompt_one(q).expect("confirmation");
        if a.as_bool().unwrap() {
            return;
        }
    }
}

/// Confirm if file must be encrypted
pub(crate) fn confirm_encryption() -> bool {
    // let the user confirm if they want the file encrypted
    let q = requestty::Question::confirm("encrypt")
        .message("Encrypt the exported key pair file?")
        .build();

    let a = requestty::prompt_one(q).expect("confirmation");
    a.as_bool().unwrap()
}

/// Request the user to input the recovery phrase
pub(crate) fn request_recovery_phrase() -> String {
    // let the user input the recovery phrase
    let q = Question::input("phrase")
        .message("Please enter the recovery phrase:")
        .validate_on_key(|phrase, _| MnemSeed::is_valid(phrase))
        .validate(|phrase, _| {
            if MnemSeed::is_valid(phrase) {
                Ok(())
            } else {
                Err("Please enter a valid recovery phrase".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q).expect("recovery phrase");
    let phrase = a.as_string().unwrap().to_string();
    phrase
}

/// Welcome the user into interactive mode and ask for an action
pub(crate) fn welcome() -> u8 {
    let q = Question::select("welcome")
        .message("What would you like to do?")
        .choices(vec![
            "Create a new wallet and store it in this computer",
            "Access a lost wallet using the recovery phrase",
        ])
        .default_separator()
        .choice("Exit")
        .build();

    let answer = requestty::prompt_one(q).expect("choice");
    match answer.as_list_item().unwrap().index {
        0 => 1,
        1 => 2,
        _ => 0,
    }
}

/// Request the user to select a wallet to open
pub(crate) fn choose_wallet(wallets: &[PathBuf]) -> Option<PathBuf> {
    let choices = wallets
        .iter()
        .filter_map(|p| p.file_stem())
        .map(|name| String::from(name.to_str().unwrap_or("Error")))
        .collect::<Vec<String>>();

    let q = Question::select("wallet")
        .message("Please choose a wallet:")
        .choices(choices)
        .default_separator()
        .choice("Other...")
        .build();
    let a = requestty::prompt_one(q).expect("choice");
    let wi = a.as_list_item().unwrap().index;

    if wi > wallets.len() {
        None
    } else {
        Some(wallets[wi].clone())
    }
}

/// Request a name for the wallet
pub(crate) fn request_wallet_name(dir: &Path) -> String {
    let q = Question::input("name")
        .message("Please enter a wallet name:")
        .validate_on_key(|name, _| !LocalStore::wallet_exists(dir, name))
        .validate(|name, _| {
            if !LocalStore::wallet_exists(dir, name) {
                Ok(())
            } else {
                Err("A wallet with this name already exists".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q).expect("wallet name");
    a.as_string().unwrap().to_string()
}

pub(crate) enum PromptCommand {
    Address(u64),
    Balance(u64),
    Transfer(u64),
    Stake(u64),
    StakeInfo(u64),
    Withdraw(u64),
    Export,
}

/// Let the user choose a command to execute
pub(crate) fn choose_command(offline: bool) -> Option<PromptCommand> {
    // notify the user if we're note connected
    let offline_notice = match offline {
        false => "",
        true => " [offline]",
    };

    let mut choices = vec!["Retrieve my public spend key"];
    let mut online_choices = vec![
        "Check my current balance",
        "Send DUSK",
        "Stake DUSK",
        "Check stake",
        "Unstake DUSK",
    ];
    if !offline {
        choices.append(&mut online_choices)
    }

    let msg = format!("What would you like to do?{}", offline_notice);
    let q = Question::select("action")
        .message(msg)
        .choices(choices)
        .default_separator()
        .choice("Export provisioner BLS key pair")
        .default_separator()
        .choice("Exit")
        .build();

    let answer = requestty::prompt_one(q).expect("command");
    let index = answer.as_list_item().unwrap().index;

    use PromptCommand::*;

    if offline {
        match index {
            0 => Some(Address(request_key_index("spend"))),
            2 => Some(Export),
            _ => None,
        }
    } else {
        match index {
            0 => Some(Address(request_key_index("spend"))),
            1 => Some(Balance(request_key_index("spend"))),
            2 => Some(Transfer(request_key_index("spend"))),
            3 => Some(Stake(request_key_index("spend"))),
            4 => Some(StakeInfo(request_key_index("stake"))),
            5 => Some(Withdraw(request_key_index("spend"))),
            7 => Some(Export),
            _ => None,
        }
    }
}

/// Let the user enter command data interactively
pub(crate) fn prepare_command(
    cmd: PromptCommand,
    balance: f64,
) -> Result<Option<CliCommand>, Error> {
    use CliCommand as Cli;
    use PromptCommand as Prompt;

    match cmd {
        // Public spend key
        Prompt::Address(key) => Ok(Some(Cli::Address { key })),
        // Check balance
        Prompt::Balance(key) => Ok(Some(Cli::Balance { key })),
        // Create transfer
        Prompt::Transfer(key) => {
            if balance == 0.0 {
                return Err(Error::NotEnoughBalance);
            }
            let cmd = Cli::Transfer {
                key,
                rcvr: request_rcvr_addr(),
                amt: request_token_amt("transfer", balance),
                gas_limit: Some(request_gas_limit()),
                gas_price: Some(request_gas_price()),
            };
            match confirm(&cmd) {
                true => Ok(Some(cmd)),
                false => Ok(None),
            }
        }
        // Stake
        Prompt::Stake(key) => {
            if balance == 0.0 {
                return Err(Error::NotEnoughBalance);
            }
            let cmd = Cli::Stake {
                key,
                stake_key: request_key_index("stake"),
                amt: request_token_amt("stake", balance),
                gas_limit: Some(request_gas_limit()),
                gas_price: Some(request_gas_price()),
            };
            match confirm(&cmd) {
                true => Ok(Some(cmd)),
                false => Ok(None),
            }
        }
        // Stake info
        Prompt::StakeInfo(key) => Ok(Some(Cli::StakeInfo { key })),
        // Withdraw stake
        Prompt::Withdraw(key) => {
            if balance == 0.0 {
                return Err(Error::NotEnoughBalance);
            }
            let cmd = Cli::WithdrawStake {
                key,
                stake_key: request_key_index("stake"),
                gas_limit: Some(request_gas_limit()),
                gas_price: Some(request_gas_price()),
            };
            match confirm(&cmd) {
                true => Ok(Some(cmd)),
                false => Ok(None),
            }
        }
        // Export BLS Key Pair
        Prompt::Export => Ok(Some(Cli::Export {
            key: request_key_index("stake"),
            plaintext: !confirm_encryption(),
        })),
    }
}

/// Request user confirmation for a trasfer transaction
fn confirm(cmd: &CliCommand) -> bool {
    use CliCommand as Cli;
    match cmd {
        Cli::Transfer {
            key: _,
            rcvr,
            amt,
            gas_limit,
            gas_price,
        } => {
            let gas_limit = gas_limit.expect("gas limit not set");
            let gas_price = gas_price.expect("gas price not set");
            let max_fee = gas_limit * gas_price;
            println!(
                "   > Recipient = {}..{}",
                &rcvr[..10],
                &rcvr[rcvr.len() - 11..]
            );
            println!("   > Amount to transfer = {} DUSK", to_dusk(amt));
            println!("   > Max fee = {} DUSK", to_dusk(&max_fee));
            ask_confirm()
        }
        Cli::Stake {
            key: _,
            stake_key,
            amt,
            gas_limit,
            gas_price,
        } => {
            let gas_limit = gas_limit.expect("gas limit not set");
            let gas_price = gas_price.expect("gas price not set");
            let max_fee = gas_limit * gas_price;
            println!("   > Stake key = {}", stake_key);
            println!("   > Amount to stake = {} DUSK", to_dusk(amt));
            println!("   > Max fee = {} DUSK", to_dusk(&max_fee));
            ask_confirm()
        }
        Cli::WithdrawStake {
            key: _,
            stake_key,
            gas_limit,
            gas_price,
        } => {
            let gas_limit = gas_limit.expect("gas limit not set");
            let gas_price = gas_price.expect("gas price not set");
            let max_fee = gas_limit * gas_price;
            println!("   > Stake key = {}", stake_key);
            println!("   > Max fee = {} DUSK", to_dusk(&max_fee));
            ask_confirm()
        }
        _ => true,
    }
}

/// Returns DUSK value of nanoDUSK amt provided
/// Note: This is only used for displaying purposes.
pub fn to_dusk(nano_dusk: &u64) -> f64 {
    let dusk = *nano_dusk as f64;
    dusk / 1e9
}

/// Asks the user for confirmation
fn ask_confirm() -> bool {
    let q = requestty::Question::confirm("confirm")
        .message("Transaction ready. Proceed?")
        .build();
    let a = requestty::prompt_one(q).expect("confirmation");
    a.as_bool().unwrap_or(false)
}

/// Request a key index from the user
fn request_key_index(key_type: &str) -> u64 {
    let question = requestty::Question::int("key")
        .message(format!("Select a {} key:", key_type))
        .default(0)
        .validate_on_key(|i, _| (0..=i64::MAX).contains(&i))
        .validate(|i, _| {
            if (0..=i64::MAX).contains(&i) {
                Ok(())
            } else {
                Err(format!("Please choose a key between 0 and {}", i64::MAX))
            }
        })
        .build();

    let a = requestty::prompt_one(question).expect("key index");
    let val = a.as_int().unwrap();
    u64::try_from(val).ok().unwrap()
}

/// Request a receiver address
fn request_rcvr_addr() -> String {
    // let the user input the receiver address
    let q = Question::input("addr")
        .message("Please enter the recipients address:")
        .validate_on_key(|addr, _| is_valid_addr(addr))
        .validate(|addr, _| {
            if is_valid_addr(addr) {
                Ok(())
            } else {
                Err("Please introduce a valid DUSK address".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q).expect("receiver address");
    a.as_string().unwrap().to_string()
}

/// Utility function to check if an address is valid
fn is_valid_addr(addr: &str) -> bool {
    !addr.is_empty() && bs58::decode(addr).into_vec().is_ok()
}

/// Checks for a valid DUSK denomination
fn check_valid_denom(num: f64, balance: f64) -> Result<(), String> {
    let min = MIN_CONVERTIBLE;
    let max = f64::min(balance, MAX_CONVERTIBLE);
    match (min..=max).contains(&num) {
        true => Ok(()),
        false => {
            Err(format!("The amount has to be between {} and {}", min, max))
        }
    }
}

/// Request amount of tokens
fn request_token_amt(action: &str, balance: f64) -> Dusk {
    let question = requestty::Question::float("amt")
        .message(format!("Introduce the amount to {}:", action))
        .default(MIN_CONVERTIBLE)
        .validate_on_key(|n, _| check_valid_denom(n, balance).is_ok())
        .validate(|n, _| check_valid_denom(n, balance))
        .build();

    let a = requestty::prompt_one(question).expect("token amount");
    let value = a.as_float().unwrap();
    dusk(value)
}

/// Request gas limit
fn request_gas_limit() -> u64 {
    let question = requestty::Question::int("amt")
        .message("Introduce the gas limit for this transaction:")
        .default(DEFAULT_GAS_LIMIT as i64)
        .validate_on_key(|n, _| n > (MIN_GAS_LIMIT as i64))
        .validate(|n, _| {
            if n < MIN_GAS_LIMIT as i64 {
                Err("Gas limit too low".to_owned())
            } else {
                Ok(())
            }
        })
        .build();

    let a = requestty::prompt_one(question).expect("gas limit");
    a.as_int().unwrap() as u64
}

/// Request gas price
fn request_gas_price() -> Dusk {
    let question = requestty::Question::float("amt")
        .message("Introduce the gas price for this transaction:")
        .default(DEFAULT_GAS_PRICE)
        .validate_on_key(|n, _| check_valid_denom(n, MAX_CONVERTIBLE).is_ok())
        .validate(|n, _| check_valid_denom(n, MAX_CONVERTIBLE))
        .build();

    let a = requestty::prompt_one(question).expect("gas price");
    let value = a.as_float().unwrap();
    dusk(value)
}

/// Request Dusk block explorer open
pub(crate) fn launch_explorer(url: String) -> bool {
    let q = requestty::Question::confirm("launch")
        .message("Launch block explorer?")
        .build();

    let a = requestty::prompt_one(q).expect("confirmation");
    let open = a.as_bool().unwrap_or(false);

    if open {
        match open::that(url) {
            Ok(()) => true,
            Err(_) => false,
        }
    } else {
        false
    }
}

/// Prints a dynamic status update
pub(crate) fn status(status: &str) {
    let filln = 26 - status.len();
    let fill = if filln > 0 {
        " ".repeat(filln)
    } else {
        "".to_string()
    };
    print!("\r{}{}", status, fill);
    let mut stdout = stdout();
    stdout.flush().unwrap();
    thread::sleep(Duration::from_millis(85));
}

/// Shows the terminal cursor
pub(crate) fn show_cursor() -> Result<(), Error> {
    let mut stdout = stdout();
    stdout.execute(Show)?;
    Ok(())
}

/// Hides the terminal cursor
pub(crate) fn hide_cursor() -> Result<(), Error> {
    let mut stdout = stdout();
    stdout.execute(Hide)?;
    Ok(())
}
