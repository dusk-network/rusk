// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::str::FromStr;
use std::{io::stdout, println};

use crossterm::{
    cursor::{Hide, Show},
    ExecutableCommand,
};

use anyhow::Result;
use bip39::{ErrorKind, Language, Mnemonic};
use execution_core::stake::MINIMUM_STAKE;
use requestty::{Choice, Question};

use rusk_wallet::gas;
use rusk_wallet::{
    currency::{Dusk, Lux},
    dat::DatFileVersion,
    Address, Error, MAX_CONVERTIBLE, MIN_CONVERTIBLE,
};
use sha2::{Digest, Sha256};

/// Request the user to authenticate with a password
pub(crate) fn request_auth(
    msg: &str,
    password: &Option<String>,
    file_version: DatFileVersion,
) -> anyhow::Result<Vec<u8>> {
    let pwd = match password.as_ref() {
        Some(p) => p.to_string(),

        None => {
            let q = Question::password("password")
                .message(format!("{}:", msg))
                .mask('*')
                .build();

            let a = requestty::prompt_one(q)?;
            a.as_string().expect("answer to be a string").into()
        }
    };

    Ok(hash(file_version, &pwd))
}

/// Request the user to create a wallet password
pub(crate) fn create_password(
    password: &Option<String>,
    file_version: DatFileVersion,
) -> anyhow::Result<Vec<u8>> {
    let pwd = match password.as_ref() {
        Some(p) => p.to_string(),
        None => {
            let mut pwd = String::from("");

            let mut pwds_match = false;
            while !pwds_match {
                // enter password
                let q = Question::password("password")
                    .message("Enter the password for the wallet:")
                    .mask('*')
                    .build();
                let a = requestty::prompt_one(q)?;
                let pwd1 = a.as_string().expect("answer to be a string");

                // confirm password
                let q = Question::password("password")
                    .message("Please confirm the password:")
                    .mask('*')
                    .build();
                let a = requestty::prompt_one(q)?;
                let pwd2 = a.as_string().expect("answer to be a string");

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

    Ok(hash(file_version, &pwd))
}

/// Display the recovery phrase to the user and ask for confirmation
pub(crate) fn confirm_recovery_phrase<S>(phrase: &S) -> anyhow::Result<()>
where
    S: std::fmt::Display,
{
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

        let a = requestty::prompt_one(q)?;
        if a.as_bool().expect("answer to be a bool") {
            return Ok(());
        }
    }
}

/// Request the user to input the recovery phrase
pub(crate) fn request_recovery_phrase() -> anyhow::Result<String> {
    // let the user input the recovery phrase
    let mut attempt = 1;
    loop {
        let q = Question::input("phrase")
            .message("Please enter the recovery phrase:")
            .build();

        let a = requestty::prompt_one(q)?;
        let phrase = a.as_string().expect("answer to be a string");

        match Mnemonic::from_phrase(phrase, Language::English) {
            Ok(phrase) => break Ok(phrase.to_string()),

            Err(err) if attempt > 2 => match err.downcast_ref::<ErrorKind>() {
                Some(ErrorKind::InvalidWord) => {
                    return Err(Error::AttemptsExhausted)?
                }
                _ => return Err(err),
            },
            Err(_) => {
                println!("Invalid recovery phrase, please try again");
                attempt += 1;
            }
        }
    }
}

fn is_valid_dir(dir: &str) -> bool {
    let mut p = std::path::PathBuf::new();
    p.push(dir);
    p.is_dir()
}

/// Use sha256 for Rusk Binary Format, and blake for the rest
fn hash(file_version: DatFileVersion, pwd: &str) -> Vec<u8> {
    match file_version {
        DatFileVersion::RuskBinaryFileFormat(_) => {
            let mut hasher = Sha256::new();

            hasher.update(pwd.as_bytes());

            hasher.finalize().to_vec()
        }
        _ => blake3::hash(pwd.as_bytes()).as_bytes().to_vec(),
    }
}

/// Request a directory
pub(crate) fn request_dir(
    what_for: &str,
    profile: PathBuf,
) -> Result<std::path::PathBuf> {
    let q = Question::input("name")
        .message(format!("Please enter a directory to {}:", what_for))
        .default(profile.as_os_str().to_str().expect("default dir"))
        .validate_on_key(|dir, _| is_valid_dir(dir))
        .validate(|dir, _| {
            if is_valid_dir(dir) {
                Ok(())
            } else {
                Err("Not a valid directory".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q)?;
    let mut p = std::path::PathBuf::new();
    p.push(a.as_string().expect("answer to be a string"));
    Ok(p)
}

/// Asks the user for confirmation
pub(crate) fn ask_confirm() -> anyhow::Result<bool> {
    let q = requestty::Question::confirm("confirm")
        .message("Transaction ready. Proceed?")
        .build();
    let a = requestty::prompt_one(q)?;
    Ok(a.as_bool().expect("answer to be a bool"))
}

/// Request a receiver address
pub(crate) fn request_rcvr_addr(addr_for: &str) -> anyhow::Result<Address> {
    // let the user input the receiver address
    let q = Question::input("addr")
        .message(format!("Please enter the {} address:", addr_for))
        .validate_on_key(|addr, _| Address::from_str(addr).is_ok())
        .validate(|addr, _| {
            if Address::from_str(addr).is_ok() {
                Ok(())
            } else {
                Err("Please introduce a valid DUSK address".to_string())
            }
        })
        .build();

    let a = requestty::prompt_one(q)?;
    Ok(Address::from_str(
        a.as_string().expect("answer to be a string"),
    )?)
}

/// Checks if the value is larger than the given min and smaller than the
/// min of the balance and `MAX_CONVERTIBLE`.
fn check_valid_denom(
    value: f64,
    min: Dusk,
    balance: Dusk,
) -> Result<(), String> {
    let value = Dusk::from(value);
    let max = std::cmp::min(balance, MAX_CONVERTIBLE);
    match (min..=max).contains(&value) {
        true => Ok(()),
        false => {
            Err(format!("The amount has to be between {} and {}", min, max))
        }
    }
}

/// Request an amount of token larger than a given min.
fn request_token(
    action: &str,
    min: Dusk,
    balance: Dusk,
) -> anyhow::Result<Dusk> {
    let question = requestty::Question::float("amt")
        .message(format!("Introduce the amount of DUSK to {}:", action))
        .default(min.into())
        .validate_on_key(|f, _| check_valid_denom(f, min, balance).is_ok())
        .validate(|f, _| check_valid_denom(f, min, balance))
        .build();

    let a = requestty::prompt_one(question)?;

    Ok(a.as_float().expect("answer to be a float").into())
}

/// Request a positive amount of tokens
pub(crate) fn request_token_amt(
    action: &str,
    balance: Dusk,
) -> anyhow::Result<Dusk> {
    let min = MIN_CONVERTIBLE;
    request_token(action, min, balance)
}

/// Request amount of tokens that can be 0
pub(crate) fn request_optional_token_amt(
    action: &str,
    balance: Dusk,
) -> anyhow::Result<Dusk> {
    let min = Dusk::from(0);
    request_token(action, min, balance)
}

/// Request amount of tokens that can't be lower than MINIMUM_STAKE
pub(crate) fn request_stake_token_amt(balance: Dusk) -> anyhow::Result<Dusk> {
    let min: Dusk = MINIMUM_STAKE.into();
    request_token("stake", min, balance)
}

/// Request gas limit
pub(crate) fn request_gas_limit(default_gas_limit: u64) -> anyhow::Result<u64> {
    let question = requestty::Question::int("amt")
        .message("Introduce the gas limit for this transaction:")
        .default(default_gas_limit as i64)
        .validate_on_key(|n, _| n > (gas::MIN_LIMIT as i64))
        .validate(|n, _| {
            if n < gas::MIN_LIMIT as i64 {
                Err("Gas limit too low".to_owned())
            } else {
                Ok(())
            }
        })
        .build();

    let a = requestty::prompt_one(question)?;
    Ok(a.as_int().expect("answer to be an int") as u64)
}

/// Request gas price
pub(crate) fn request_gas_price() -> anyhow::Result<Lux> {
    let question = requestty::Question::float("amt")
        .message("Introduce the gas price for this transaction:")
        .default(Dusk::from(gas::DEFAULT_PRICE).into())
        .validate_on_key(|f, _| {
            check_valid_denom(f, MIN_CONVERTIBLE, MAX_CONVERTIBLE).is_ok()
        })
        .validate(|f, _| check_valid_denom(f, MIN_CONVERTIBLE, MAX_CONVERTIBLE))
        .build();

    let a = requestty::prompt_one(question)?;
    let price = Dusk::from(a.as_float().expect("answer to be a float"));
    Ok(*price)
}

pub(crate) fn request_str(name: &str) -> anyhow::Result<String> {
    let question = requestty::Question::input("string")
        .message(format!("Introduce string for {}:", name))
        .build();

    let a = requestty::prompt_one(question)?;
    Ok(a.as_string().expect("answer to be a string").to_owned())
}

pub enum TransactionModel {
    Shielded,
    Public,
}

impl From<&str> for TransactionModel {
    fn from(value: &str) -> Self {
        match value {
            "Shielded" => TransactionModel::Shielded,
            "Public" => TransactionModel::Public,
            _ => panic!("Unknown transaction model"),
        }
    }
}

/// Request transaction model to use
pub(crate) fn request_transaction_model() -> anyhow::Result<TransactionModel> {
    let question = requestty::Question::select(
        "Please specify the transaction model to use",
    )
    .choices(vec![Choice("Public".into()), "Shielded".into()])
    .build();

    let a = requestty::prompt_one(question)?;
    Ok(a.as_list_item()
        .expect("answer must be a list item")
        .text
        .as_str()
        .into())
}

/// Request contract WASM file location
pub(crate) fn request_contract_code() -> anyhow::Result<PathBuf> {
    let question = requestty::Question::input("Location of the WASM contract")
        .message("Location of the WASM file:")
        .validate_on_key(|f, _| PathBuf::from(f).exists())
        .validate(|f, _| {
            PathBuf::from(f)
                .exists()
                .then_some(())
                .ok_or("File not found".to_owned())
        })
        .build();

    let a = requestty::prompt_one(question)?;
    let location = a.as_string().expect("answer to be a string").to_owned();

    Ok(PathBuf::from(location))
}

pub(crate) fn request_bytes(name: &str) -> anyhow::Result<Vec<u8>> {
    let question = requestty::Question::input("bytes")
        .message(format!("Introduce bytes for {}", name))
        .validate_on_key(|f, _| hex::decode(f).is_ok())
        .validate(|f, _| {
            hex::decode(f)
                .is_ok()
                .then_some(())
                .ok_or("Invalid hex string".to_owned())
        })
        .build();

    let a = requestty::prompt_one(question)?;
    let bytes = hex::decode(a.as_string().expect("answer to be a string"))?;

    Ok(bytes)
}

pub(crate) fn request_nonce() -> anyhow::Result<u64> {
    let question = requestty::Question::input("Contract Deployment nonce")
        .message("Introduce a number for nonce")
        .validate_on_key(|f, _| u64::from_str(f).is_ok())
        .validate(|f, _| {
            u64::from_str(f)
                .is_ok()
                .then_some(())
                .ok_or("Invalid number".to_owned())
        })
        .build();

    let a = requestty::prompt_one(question)?;
    let bytes = u64::from_str(a.as_string().expect("answer to be a string"))?;

    Ok(bytes)
}

/// Request Dusk block explorer to be opened
pub(crate) fn launch_explorer(url: String) -> Result<()> {
    let q = requestty::Question::confirm("launch")
        .message("Launch block explorer?")
        .build();

    let a = requestty::prompt_one(q)?;
    let open = a.as_bool().expect("answer to be a bool");
    if open {
        open::that(url)?;
    }
    Ok(())
}

/// Shows the terminal cursor
pub(crate) fn show_cursor() -> anyhow::Result<()> {
    let mut stdout = stdout();
    stdout.execute(Show)?;
    Ok(())
}

/// Hides the terminal cursor
pub(crate) fn hide_cursor() -> anyhow::Result<()> {
    let mut stdout = stdout();
    stdout.execute(Hide)?;
    Ok(())
}
