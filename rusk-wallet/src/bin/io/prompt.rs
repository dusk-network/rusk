// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, BufRead, Write, stdout};
use std::println;

use bip39::{ErrorKind, Language, Mnemonic};
use crossterm::ExecutableCommand;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use rusk_wallet::dat::{
    FileVersion as DatFileVersion, version_without_pre_higher,
};
use rusk_wallet::{Error, PBKDF2_ROUNDS, SALT_SIZE};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;
use zeroize::{Zeroize, Zeroizing};

use crate::command::TransactionHistory;

#[derive(Debug, ThisError)]
pub(crate) enum PromptAbort {
    #[error("interrupted")]
    Interrupted,
    #[error("cancelled")]
    Cancelled,
}

pub(crate) trait Prompt {
    fn create_new_password(&self) -> anyhow::Result<String> {
        create_new_password()
    }

    fn prompt_text(&self, message: &str) -> anyhow::Result<String> {
        read_line(message)
    }
}

pub(crate) struct Prompter;

impl Prompt for Prompter {}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Read a masked password from the terminal using crossterm raw mode.
fn read_password(prompt: &str) -> anyhow::Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;

    enable_raw_mode()?;
    let _raw_mode_guard = RawModeGuard;
    (|| {
        let mut pwd = String::new();
        loop {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        eprintln!();
                        return Ok(pwd);
                    }
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        pwd.zeroize();
                        eprintln!();
                        anyhow::bail!(PromptAbort::Interrupted);
                    }
                    KeyCode::Esc => {
                        pwd.zeroize();
                        eprintln!();
                        anyhow::bail!(PromptAbort::Cancelled);
                    }
                    KeyCode::Char(c) => {
                        pwd.push(c);
                        eprint!("*");
                        io::stderr().flush()?;
                    }
                    KeyCode::Backspace => {
                        if pwd.pop().is_some() {
                            eprint!("\x08 \x08");
                            io::stderr().flush()?;
                        }
                    }
                    _ => {}
                }
            }
        }
    })()
}

/// Read a line of text from stdin.
fn read_line(prompt: &str) -> anyhow::Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Ask a yes/no confirmation question.
fn confirm(msg: &str) -> anyhow::Result<bool> {
    loop {
        eprint!("{msg} [y/n] ");
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => eprintln!("Please answer y or n."),
        }
    }
}

pub(crate) fn ask_pwd(msg: &str) -> anyhow::Result<String> {
    read_password(msg)
}

pub(crate) fn create_new_password() -> anyhow::Result<String> {
    loop {
        let mut pwd = read_password("Password: ")?;
        let mut confirm_pwd = read_password("Confirm password: ")?;
        if pwd == confirm_pwd {
            confirm_pwd.zeroize();
            return Ok(pwd);
        }
        confirm_pwd.zeroize();
        pwd.zeroize();
        eprintln!("Passwords don't match. Try again.");
    }
}

/// Request the user to authenticate with a password and return the derived key
pub(crate) fn derive_key_from_password(
    msg: &str,
    password: &Option<String>,
    salt: Option<&[u8; SALT_SIZE]>,
    file_version: DatFileVersion,
) -> anyhow::Result<Vec<u8>> {
    let mut pwd = match password.as_ref() {
        Some(p) => p.to_string(),
        None => ask_pwd(msg)?,
    };

    let key = derive_key(file_version, &pwd, salt);
    pwd.zeroize();
    key
}

/// Request the user to create a wallet password and return the derived key
pub(crate) fn derive_key_from_new_password(
    password: &Option<String>,
    salt: Option<&[u8; SALT_SIZE]>,
    file_version: DatFileVersion,
    prompter: &dyn Prompt,
) -> anyhow::Result<Vec<u8>> {
    let mut pwd = match password.as_ref() {
        Some(p) => p.to_string(),
        None => prompter.create_new_password()?,
    };

    let key = derive_key(file_version, &pwd, salt);
    pwd.zeroize();
    key
}

/// Display the mnemonic phrase to the user and ask for confirmation
pub(crate) fn confirm_mnemonic_phrase<S>(phrase: &S) -> anyhow::Result<()>
where
    S: std::fmt::Display,
{
    eprintln!(
        "The following phrase is essential for you to regain access to your wallet\n\
         in case you lose access to this computer. Please print it or write it\n\
         down and store it somewhere safe.\n\
         > {phrase}"
    );

    while !confirm("Have you backed up this phrase?")? {}

    Ok(())
}

/// Request the user to input the mnemonic phrase
pub(crate) fn request_mnemonic_phrase(
    prompter: &dyn Prompt,
) -> anyhow::Result<Zeroizing<String>> {
    let mut attempt = 1;
    loop {
        let mut phrase =
            prompter.prompt_text("Please enter the mnemonic phrase: ")?;

        match Mnemonic::from_phrase(&phrase, Language::English) {
            Ok(mnem) => {
                let validated_phrase = Zeroizing::new(mnem.into_phrase());
                phrase.zeroize();
                break Ok(validated_phrase);
            }
            Err(err) if attempt > 2 => {
                phrase.zeroize();
                match err.downcast_ref::<ErrorKind>() {
                    Some(ErrorKind::InvalidWord) => {
                        Err(Error::AttemptsExhausted)?
                    }
                    _ => return Err(err),
                }
            }
            Err(_) => {
                phrase.zeroize();
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

/// Asks the user for confirmation before deleting cache
pub(crate) fn ask_confirm_erase_cache(msg: &str) -> anyhow::Result<bool> {
    confirm(msg)
}

pub(crate) fn tx_history_list(
    history: &[TransactionHistory],
) -> anyhow::Result<()> {
    if history.is_empty() {
        println!("No transactions found");
        return Ok(());
    }
    println!("{}", TransactionHistory::header());
    for entry in history {
        println!("{entry}");
    }
    Ok(())
}

/// Shows the terminal cursor
pub(crate) fn show_cursor() -> anyhow::Result<()> {
    let mut stdout = stdout();
    stdout.execute(Show)?;
    Ok(())
}
