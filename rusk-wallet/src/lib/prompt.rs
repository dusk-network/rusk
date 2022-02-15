// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::path::PathBuf;

use blake3::Hash;
use requestty::Question;

use crate::lib::crypto::MnemSeed;
use crate::lib::{
    to_udusk, Dusk, MicroDusk, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE,
    MAX_CONVERTIBLE_DUSK, ONE_MICRO_DUSK,
};
use crate::{CliCommand, WalletCfg};

/// Request the user to authenticate with a password
pub(crate) fn request_auth(msg: &str) -> Hash {
    let pwd = match env::var("RUSK_WALLET_PWD") {
        Ok(p) => p,
        Err(_) => {
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
pub(crate) fn select_wallet(
    dir: &str,
    wallets: Vec<String>,
) -> Option<PathBuf> {
    // select the wallet
    let q = Question::select("wallet")
        .message("Please select the wallet you wish to use:")
        .choices(&wallets)
        .default_separator()
        .choice("Other...")
        .build();
    let a = requestty::prompt_one(q).expect("choice");
    let wi = a.as_list_item().unwrap().index;

    if wi > wallets.len() {
        None
    } else {
        // gen full path for selected wallet
        let mut path = PathBuf::new();
        path.push(dir);
        path.push(wallets[wi].clone());
        Some(path)
    }
}

/// Request a name for the wallet
pub(crate) fn request_wallet_name() -> String {
    let q = Question::input("name")
        .message("Please enter a wallet name:")
        .validate_on_key(|name, _| !wallet_exists(name))
        .validate(|name, _| {
            if !wallet_exists(name) {
                Ok(())
            } else {
                Err("A wallet with this name already exists".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q).expect("wallet name");
    a.as_string().unwrap().to_string()
}

/// Checks if a wallet with this name already exists
fn wallet_exists(name: &str) -> bool {
    let mut pb = PathBuf::new();
    pb.push(WalletCfg::default_data_dir());
    pb.push(name);
    pb.set_extension("dat");
    pb.is_file()
}

pub(crate) enum PromptCommand {
    Address(u64),
    Balance(u64),
    Transfer(u64),
    Stake(u64),
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
        "Send Dusk",
        "Stake Dusk",
        "Unstake Dusk",
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
            4 => Some(Withdraw(request_key_index("spend"))),
            6 => Some(Export),
            _ => None,
        }
    }
}

pub(crate) fn prepare_command(cmd: PromptCommand, balance: Dusk) -> CliCommand {
    use CliCommand as Cli;
    use PromptCommand as Prompt;

    match cmd {
        // Public spend key
        Prompt::Address(key) => Cli::Address { key },
        // Check balance
        Prompt::Balance(key) => Cli::Balance { key },
        // Create transfer
        Prompt::Transfer(key) => {
            let rcvr = request_rcvr_addr();
            let amt = request_token_amt("transfer", balance);
            let gas_limit = request_gas_limit();
            let gas_price = Some(request_gas_price());
            Cli::Transfer {
                key,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            }
        }
        // Stake
        Prompt::Stake(key) => {
            let stake_key = request_key_index("stake");
            let amt = request_token_amt("stake", balance);
            let gas_limit = request_gas_limit();
            let gas_price = Some(request_gas_price());
            Cli::Stake {
                key,
                stake_key,
                amt,
                gas_limit,
                gas_price,
            }
        }
        // Withdraw stake
        Prompt::Withdraw(key) => {
            let stake_key = request_key_index("stake");
            let gas_limit = request_gas_limit();
            let gas_price = Some(request_gas_price());
            Cli::WithdrawStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            }
        }
        // Export BLS Key Pair
        Prompt::Export => {
            let key = request_key_index("stake");
            let encrypt = confirm_encryption();
            Cli::Export {
                key,
                plaintext: !encrypt,
            }
        }
    }
}

/// Request a key index from the user
pub(crate) fn request_key_index(key_type: &str) -> u64 {
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
pub(crate) fn request_rcvr_addr() -> String {
    // let the user input the receiver address
    let q = Question::input("addr")
        .message("Please enter the recipients address:")
        .validate_on_key(|addr, _| is_valid_addr(addr))
        .validate(|addr, _| {
            if is_valid_addr(addr) {
                Ok(())
            } else {
                Err("Please introduce a valid Dusk address".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q).expect("receiver address");
    a.as_string().unwrap().to_string()
}

/// Utility function to check if an address is valid
fn is_valid_addr(addr: &str) -> bool {
    bs58::decode(addr).into_vec().is_ok()
}

fn check_valid_denom(num: Dusk, balance: Dusk) -> Result<(), String> {
    if num.is_finite() && num > 0.0 {
        if num > MAX_CONVERTIBLE_DUSK {
            return Err(format!(
                "This is greater than the max amount allowed {}",
                MAX_CONVERTIBLE_DUSK
            ));
        }
        if num < ONE_MICRO_DUSK {
            return Err(format!(
                "Input too low, please increase at least to {}",
                ONE_MICRO_DUSK
            ));
        }
        if num < balance {
            Ok(())
        } else {
            Err("insufficient balance".to_owned())
        }
    } else {
        Err("invalid denomination".to_owned())
    }
}

/// Request amount of tokens
pub(crate) fn request_token_amt(action: &str, balance: Dusk) -> MicroDusk {
    let question = requestty::Question::float("amt")
        .message(format!("Introduce the amount to {}:", action))
        .default(Dusk::default())
        .validate_on_key(|n, _| check_valid_denom(n, balance).is_ok())
        .validate(|n, _| check_valid_denom(n, balance))
        .build();

    let a = requestty::prompt_one(question).expect("token amount");
    let dusk = a.as_float().unwrap();
    to_udusk(dusk)
}

/// Request gas limit
pub(crate) fn request_gas_limit() -> u64 {
    let question = requestty::Question::int("amt")
        .message("Introduce the gas limit for this transaction:")
        .default(DEFAULT_GAS_LIMIT as i64)
        .validate_on_key(|n, _| n > 0)
        .validate(|n, _| {
            if n > 0 {
                Ok(())
            } else {
                Err("invalid gas limit".to_owned())
            }
        })
        .build();

    let a = requestty::prompt_one(question).expect("gas limit");
    a.as_int().unwrap() as u64
}

/// Request gas price
pub(crate) fn request_gas_price() -> MicroDusk {
    let question = requestty::Question::float("amt")
        .message("Introduce the gas price for this transaction:")
        .default(DEFAULT_GAS_PRICE)
        .validate_on_key(|n, _| check_valid_denom(n, Dusk::MAX).is_ok())
        .validate(|n, _| check_valid_denom(n, Dusk::MAX))
        .build();

    let a = requestty::prompt_one(question).expect("gas price");
    let dusk = a.as_float().unwrap();
    to_udusk(dusk)
}
