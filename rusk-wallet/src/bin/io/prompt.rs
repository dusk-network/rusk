// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use std::{io::stdout, println};

use crossterm::{
    cursor::{Hide, Show},
    ExecutableCommand,
};

use anyhow::Result;
use bip39::{ErrorKind, Language, Mnemonic};

use inquire::ui::RenderConfig;
use inquire::validator::Validation;
use inquire::{
    Confirm, CustomType, InquireError, Password, PasswordDisplayMode, Select,
    Text,
};
use rusk_wallet::dat::version_without_pre_higher;
use rusk_wallet::{
    currency::{Dusk, Lux},
    dat::FileVersion as DatFileVersion,
    gas::{self, MempoolGasPrices},
    Address, Error, MAX_CONVERTIBLE, MIN_CONVERTIBLE,
};
use rusk_wallet::{PBKDF2_ROUNDS, SALT_SIZE};
use sha2::{Digest, Sha256};

use crate::command::TransactionHistory;

pub(crate) fn ask_pwd(msg: &str) -> Result<String, InquireError> {
    let pwd = Password::new(msg)
        .with_display_toggle_enabled()
        .without_confirmation()
        .with_display_mode(PasswordDisplayMode::Masked)
        .prompt();

    pwd
}

pub(crate) fn create_new_password() -> Result<String, InquireError> {
    let pwd = Password::new("Password:")
        .with_display_toggle_enabled()
        .with_display_mode(PasswordDisplayMode::Hidden)
        .with_custom_confirmation_message("Confirm password: ")
        .with_custom_confirmation_error_message("The passwords doesn't match")
        .prompt();

    pwd
}

/// Request the user to authenticate with a password and return the derived key
pub(crate) fn derive_key_from_password(
    msg: &str,
    password: &Option<String>,
    salt: Option<&[u8; SALT_SIZE]>,
    file_version: DatFileVersion,
) -> anyhow::Result<Vec<u8>> {
    let pwd = match password.as_ref() {
        Some(p) => p.to_string(),

        None => ask_pwd(msg)?,
    };

    derive_key(file_version, &pwd, salt)
}

/// Request the user to create a wallet password and return the derived key
pub(crate) fn derive_key_from_new_password(
    password: &Option<String>,
    salt: Option<&[u8; SALT_SIZE]>,
    file_version: DatFileVersion,
) -> anyhow::Result<Vec<u8>> {
    let pwd = match password.as_ref() {
        Some(p) => p.to_string(),
        None => create_new_password()?,
    };

    derive_key(file_version, &pwd, salt)
}

/// Display the mnemonic phrase to the user and ask for confirmation
pub(crate) fn confirm_mnemonic_phrase<S>(phrase: &S) -> anyhow::Result<()>
where
    S: std::fmt::Display,
{
    // inform the user about the mnemonic phrase
    let msg = format!("The following phrase is essential for you to regain access to your wallet\nin case you lose access to this computer. Please print it or write it down and store it somewhere safe.\n> {} \nHave you backed up this phrase?", phrase);

    // let the user confirm they have backed up their phrase
    let confirm = Confirm::new(&msg)
        .with_help_message(
            "It is important you backup the mnemonic phrase before proceeding",
        )
        .prompt()?;

    if !confirm {
        confirm_mnemonic_phrase(phrase)?
    }

    Ok(())
}

/// Request the user to input the mnemonic phrase
pub(crate) fn request_mnemonic_phrase() -> anyhow::Result<String> {
    // let the user input the mnemonic phrase
    let mut attempt = 1;
    loop {
        let phrase =
            Text::new("Please enter the mnemonic phrase: ").prompt()?;

        match Mnemonic::from_phrase(&phrase, Language::English) {
            Ok(phrase) => break Ok(phrase.to_string()),

            Err(err) if attempt > 2 => match err.downcast_ref::<ErrorKind>() {
                Some(ErrorKind::InvalidWord) => {
                    return Err(Error::AttemptsExhausted)?
                }
                _ => return Err(err),
            },
            Err(_) => {
                println!("Invalid mnemonic phrase, please try again");
                attempt += 1;
            }
        }
    }
}

pub(crate) fn derive_key(
    file_version: DatFileVersion,
    pwd: &str,
    salt: Option<&[u8; SALT_SIZE]>,
) -> anyhow::Result<Vec<u8>> {
    match file_version {
        DatFileVersion::RuskBinaryFileFormat(version) => {
            if version_without_pre_higher(version) >= (0, 0, 2, 0) {
                let salt = salt
                    .ok_or_else(|| anyhow::anyhow!("Couldn't find the salt"))?;
                Ok(pbkdf2::pbkdf2_hmac_array::<Sha256, SALT_SIZE>(
                    pwd.as_bytes(),
                    salt,
                    PBKDF2_ROUNDS,
                )
                .to_vec())
            } else {
                let mut hasher = Sha256::new();
                hasher.update(pwd.as_bytes());
                Ok(hasher.finalize().to_vec())
            }
        }
        _ => Ok(blake3::hash(pwd.as_bytes()).as_bytes().to_vec()),
    }
}

/// Request a directory
pub(crate) fn request_dir(
    what_for: &str,
    profile: PathBuf,
) -> Result<std::path::PathBuf> {
    let validator = |dir: &str| {
        let path = PathBuf::from(dir);

        if path.is_dir() {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid("Not a valid directory".into()))
        }
    };

    let msg = format!("Please enter a directory to {}:", what_for);
    let q = match profile.to_str() {
        Some(p) => Text::new(msg.as_str())
            .with_default(p)
            .with_validator(validator)
            .prompt(),
        None => Text::new(msg.as_str()).with_validator(validator).prompt(),
    }?;

    let p = PathBuf::from(q);

    Ok(p)
}

/// Asks the user for confirmation
pub(crate) fn ask_confirm() -> anyhow::Result<bool> {
    Ok(Confirm::new("Transaction ready. Proceed?")
        .with_default(true)
        .prompt()?)
}

/// Asks the user for confirmation before deleting cache
pub(crate) fn ask_confirm_erase_cache(msg: &str) -> anyhow::Result<bool> {
    Ok(Confirm::new(msg).prompt()?)
}

/// Request a receiver address
pub(crate) fn request_rcvr_addr(addr_for: &str) -> anyhow::Result<Address> {
    // let the user input the receiver address
    Ok(Address::from_str(
        &Text::new(format!("Please enter the {} address:", addr_for).as_str())
            .with_validator(|addr: &str| {
                if Address::from_str(addr).is_ok() {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(
                        "Please introduce a valid DUSK address".into(),
                    ))
                }
            })
            .prompt()?,
    )?)
}

/// Request an amount of token larger than a given min.
fn request_token(
    action: &str,
    min: Dusk,
    balance: Dusk,
    default: Option<f64>,
) -> Result<Dusk, Error> {
    // Checks if the value is larger than the given min and smaller than the
    // min of the balance and `MAX_CONVERTIBLE`.
    let validator = move |value: &f64| {
        let max = std::cmp::min(balance, MAX_CONVERTIBLE);

        match (min..=max).contains(&Dusk::try_from(*value)?) {
            true => Ok(Validation::Valid),
            false => Ok(Validation::Invalid(
                format!("The amount has to be between {} and {}", min, max)
                    .into(),
            )),
        }
    };

    let msg = format!("Introduce dusk amount for {}:", action);

    let amount_prompt: CustomType<f64> = CustomType {
        message: &msg,
        starting_input: None,
        formatter: &|i| format!("{} DUSK", i),
        default_value_formatter: &|i| format!("{} DUSK", i),
        default,
        validators: vec![Box::new(validator)],
        placeholder: Some("123.45"),
        error_message: "Please type a valid number.".into(),
        help_message: "The number should use a dot as the decimal separator."
            .into(),
        parser: &|i| match i.parse::<f64>() {
            Ok(val) => Ok(val),
            Err(_) => Err(()),
        },
        render_config: RenderConfig::default(),
    };

    amount_prompt.prompt()?.try_into()
}

/// Request a positive amount of tokens
pub(crate) fn request_token_amt(
    action: &str,
    balance: Dusk,
) -> Result<Dusk, Error> {
    let min = MIN_CONVERTIBLE;

    request_token(action, min, balance, None).map_err(Error::from)
}

/// Request amount of tokens that can be 0
pub(crate) fn request_optional_token_amt(
    action: &str,
    balance: Dusk,
) -> Result<Dusk, Error> {
    let min = Dusk::from(0);

    request_token(action, min, balance, None).map_err(Error::from)
}

/// Request amount of tokens that can't be lower than the `min` argument and
/// higher than `balance`
pub(crate) fn request_stake_token_amt(
    balance: Dusk,
    min: Dusk,
) -> Result<Dusk, Error> {
    request_token("stake", min, balance, None).map_err(Error::from)
}

/// Request gas limit
pub(crate) fn request_gas_limit(default_gas_limit: u64) -> anyhow::Result<u64> {
    Ok(
        CustomType::<u64>::new("Introduce the gas limit for this transaction:")
            .with_default(default_gas_limit)
            .with_validator(|n: &u64| {
                if *n < gas::MIN_LIMIT {
                    Ok(Validation::Invalid("Gas limit too low".into()))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?,
    )
}

/// Request gas price
pub(crate) fn request_gas_price(
    min_gas_price: Lux,
    mempool_gas_prices: MempoolGasPrices,
) -> Result<Lux, Error> {
    let default_gas_price = if mempool_gas_prices.average > min_gas_price {
        mempool_gas_prices.average
    } else {
        min_gas_price
    };

    Ok(
        CustomType::<u64>::new("Introduce the gas price for this transaction:")
            .with_default(default_gas_price)
            .with_formatter(&|val| format!("{} LUX", val))
            .prompt()?,
    )
}

pub(crate) fn request_init_args() -> anyhow::Result<Vec<u8>> {
    const MAX_INIT_SIZE: usize = 32 * 1024;
    let init = Text::new("Introduce init args:")
        .with_help_message("Hex encoded rkyv serialized data")
        .with_validator(move |input: &str| {
            let error = match hex::decode(input) {
                Ok(data) => data.len().gt(&MAX_INIT_SIZE).then_some(format!(
                    "Input exceeds the maximum size of {MAX_INIT_SIZE} bytes",
                )),
                Err(_) => Some("Data must be a valid hex".into()),
            };
            Ok(error.map_or(Validation::Valid, |error| {
                Validation::Invalid(error.into())
            }))
        })
        .prompt()?;
    let init = hex::decode(init).map_err(|e| {
        anyhow::anyhow!("Expecting hex, this should be a bug: {e}")
    })?;

    Ok(init)
}

pub(crate) fn request_str(
    name: &str,
    max_length: usize,
) -> anyhow::Result<String> {
    Ok(
        Text::new(format!("Introduce string for {}:", name).as_str())
            .with_validator(move |input: &str| {
                if input.len() > max_length {
                    Ok(Validation::Invalid(
                        format!(
                            "Input exceeds the maximum length of {} characters",
                            max_length
                        )
                        .into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?,
    )
}

pub enum TransactionModel {
    Shielded,
    Public,
}

impl Display for TransactionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransactionModel::Shielded => write!(f, "Shielded"),
            TransactionModel::Public => write!(f, "Public"),
        }
    }
}

/// Request transaction model to use
pub(crate) fn request_transaction_model() -> anyhow::Result<TransactionModel> {
    let choices = vec![TransactionModel::Shielded, TransactionModel::Public];

    Ok(
        Select::new("Please specify the transaction model to use", choices)
            .prompt()?,
    )
}

/// Request transaction model to use
pub(crate) fn request_address(
    current_idx: u8,
    choices: Vec<Address>,
) -> anyhow::Result<Address> {
    Ok(Select::new(
        "Please select the moonlight address to use as stake owner",
        choices,
    )
    .with_starting_cursor(current_idx as usize)
    .prompt()?)
}

pub(crate) fn tx_history_list(
    history: &[TransactionHistory],
) -> anyhow::Result<()> {
    let header = TransactionHistory::header();
    let history_str: Vec<String> =
        history.iter().map(|history| history.to_string()).collect();

    Select::new(header.as_str(), history_str)
        .with_help_message("↑↓ to move, type to filter")
        .prompt()?;

    Ok(())
}

/// Request contract WASM file location
pub(crate) fn request_contract_code() -> anyhow::Result<PathBuf> {
    let validator = |path_str: &str| {
        let path = PathBuf::from(path_str);
        if path.extension().map_or(false, |ext| ext == "wasm") {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid("Not a valid directory".into()))
        }
    };

    let q = Text::new("Please Enter location of the WASM contract:")
        .with_validator(validator)
        .prompt()?;

    let p = PathBuf::from(q);

    Ok(p)
}

pub(crate) fn request_bytes(name: &str) -> anyhow::Result<Vec<u8>> {
    let byte_string =
        Text::new(format!("Introduce hex bytes for {}:", name).as_str())
            .with_validator(|f: &str| match hex::decode(f) {
                Ok(_) => Ok(Validation::Valid),
                Err(_) => Ok(Validation::Invalid("Invalid hex string".into())),
            })
            .prompt()?;

    let bytes = hex::decode(byte_string)?;

    Ok(bytes)
}

pub(crate) fn request_nonce() -> anyhow::Result<u64> {
    let nonce_string =
        Text::new("Introduce a number for Contract Deployment nonce:")
            .with_validator(|f: &str| match u64::from_str(f) {
                Ok(_) => Ok(Validation::Valid),
                Err(_) => Ok(Validation::Invalid("Invalid u64 nonce".into())),
            })
            .prompt()?;

    let bytes = u64::from_str(&nonce_string)?;

    Ok(bytes)
}

/// Request Dusk block explorer to be opened
pub(crate) fn launch_explorer(url: String) -> Result<()> {
    if Confirm::new("Launch block explorer?").prompt()? {
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
