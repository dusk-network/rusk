// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use blake3::Hash;
use requestty::Question;

use crate::CliCommand;

/// Request the user to authenticate with a password
pub(crate) fn request_auth() -> Hash {
    let q = Question::password("password")
        .message("Please enter your wallet's password:")
        .mask('*')
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let pwd = a.as_string().unwrap_or("").to_string();
    let pwd = blake3::hash(pwd.as_bytes());
    pwd
}

/// Request the user to create a wallet password
pub(crate) fn create_password() -> Hash {
    let mut pwd = String::from("");

    let mut pwds_match = false;
    while !pwds_match {
        // enter password
        let q = Question::password("password")
            .message("Enter a strong password for your wallet:")
            .mask('*')
            .build();
        let a = requestty::prompt_one(q).unwrap();
        let pwd1 = a.as_string().unwrap_or("").to_string();

        // confirm password
        let q = Question::password("password")
            .message("Please confirm your password:")
            .mask('*')
            .build();
        let a = requestty::prompt_one(q).unwrap();
        let pwd2 = a.as_string().unwrap_or("").to_string();

        // check match
        pwds_match = pwd1 == pwd2;
        if pwds_match {
            pwd = pwd1.to_string()
        } else {
            println!("Passwords don't match, please try again.");
        }
    }

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
        let a = requestty::prompt_one(q).unwrap();
        if a.as_bool().unwrap() {
            return;
        }
    }
}

/// Request the user to input the recovery phrase
pub(crate) fn request_recovery_phrase() -> String {
    // let the user input the recovery phrase
    let q = Question::input("phrase")
        .message("Please enter the recovery phrase:")
        .build();
    let a = requestty::prompt_one(q).unwrap();
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
    let answer = requestty::prompt_one(q).unwrap();
    match answer.as_list_item().unwrap().index {
        0 => 1,
        1 => 2,
        _ => 0,
    }
}

/// Request the user to select a wallet to open
pub(crate) fn select_wallet(dir: &str, wallets: Vec<String>) -> PathBuf {
    // select the wallet
    let q = Question::select("wallet")
        .message("Please select the wallet you wish to use:")
        .choices(&wallets)
        .build();
    let a = requestty::prompt_one(q).unwrap();
    let wi = a.as_list_item().unwrap().index;

    // gen full path for selected wallet
    let mut path = PathBuf::new();
    path.push(dir);
    path.push(wallets[wi].clone());
    path
}

/// Let the user choose a command to execute
pub(crate) fn command() -> Option<CliCommand> {
    let msg = "What would you like to do?";
    let q = Question::select("action")
        .message(msg)
        .choices(vec![
            "Check my current balance",
            "Retrieve my public spend key",
            "Send Dusk",
            "Stake Dusk",
            "Extend stake for a particular key",
            "Withdraw a key's stake",
        ])
        .default_separator()
        .choice("Exit")
        .build();

    let answer = requestty::prompt_one(q).unwrap();

    use CliCommand::*;
    match answer.as_list_item().unwrap().index {
        // Check balance
        0 => {
            let key = request_key_index("spend");
            Some(Balance { key })
        }
        // Public spend key
        1 => {
            let key = request_key_index("spend");
            Some(Address { key })
        }
        // Create transfer
        2 => {
            let key = request_key_index("spend");
            let rcvr = request_rcvr_addr();
            let amt = request_token_amt("transfer");
            let gas_limit = request_gas_limit();
            let gas_price = Some(0);
            Some(Transfer {
                key,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            })
        }
        // Stake
        3 => {
            let key = request_key_index("spend");
            let stake_key = request_key_index("stake");
            let amt = request_token_amt("stake");
            let gas_limit = request_gas_limit();
            let gas_price = Some(0);
            Some(Stake {
                key,
                stake_key,
                amt,
                gas_limit,
                gas_price,
            })
        }
        // Extend stake
        5 => {
            let key = request_key_index("spend");
            let stake_key = request_key_index("stake");
            let gas_limit = request_gas_limit();
            let gas_price = Some(0);
            Some(ExtendStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            })
        }
        // Withdraw stake
        6 => {
            let key = request_key_index("spend");
            let stake_key = request_key_index("stake");
            let gas_limit = request_gas_limit();
            let gas_price = Some(0);
            Some(WithdrawStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            })
        }
        _ => None,
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

    let a = requestty::prompt_one(question).unwrap();
    let val = a.as_int().unwrap();
    u64::try_from(val).ok().unwrap()
}

/// Request a receiver address
pub(crate) fn request_rcvr_addr() -> String {
    // let the user input the recovery phrase
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

    let a = requestty::prompt_one(q).unwrap();
    a.as_string().unwrap().to_string()
}

/// Utility function to check if an address is valid
fn is_valid_addr(addr: &str) -> bool {
    bs58::decode(addr).into_vec().is_ok()
}

/// Request amount of tokens
pub(crate) fn request_token_amt(action: &str) -> u64 {
    let question = requestty::Question::float("amt")
        .message(format!("Introduce the amount to {} (Dusk):", action))
        .default(0.0)
        .validate(|num, _| {
            if num.is_finite() && num.is_sign_positive() {
                Ok(())
            } else {
                Err("Please enter a finite number".to_owned())
            }
        })
        .build();

    let a = requestty::prompt_one(question).unwrap();
    let dusk_amt = a.as_float().unwrap();
    (dusk_amt * 1_000_000.0) as u64
}

/// Request gas spend limit
pub(crate) fn request_gas_limit() -> u64 {
    let question = requestty::Question::int("amt")
        .message("Introduce the gas spend limit for this transaction (µDusk):")
        .default(0)
        .validate_on_key(|i, _| (0..=i64::MAX).contains(&i))
        .validate(|i, _| {
            if (0..=i64::MAX).contains(&i) {
                Ok(())
            } else {
                Err(format!(
                    "Please introduce an amount between 0 and {}",
                    i64::MAX
                ))
            }
        })
        .build();

    let a = requestty::prompt_one(question).unwrap();
    let val = a.as_int().unwrap();
    u64::try_from(val).ok().unwrap()
}
