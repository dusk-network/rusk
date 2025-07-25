// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod command;
mod config;
mod interactive;
mod io;
mod settings;

use command::{gen_iv, gen_salt};
pub(crate) use command::{Command, RunResult};
use io::prompt::{ask_pwd, derive_key, Prompter};
use zeroize::Zeroize;

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use inquire::InquireError;
use rocksdb::ErrorKind;
use rusk_wallet::currency::Dusk;
use rusk_wallet::dat::{self, FileVersion as DatFileVersion, LATEST_VERSION};
use rusk_wallet::{
    Error, GraphQL, Profile, SecureWalletFile, Wallet, WalletPath, EPOCH,
    IV_SIZE, SALT_SIZE,
};
use tracing::{error, info, warn, Level};

use crate::settings::{LogFormat, Settings};

use config::Config;
use io::{prompt, status, WalletArgs};

#[derive(Debug, Clone)]
pub(crate) struct WalletFile {
    path: WalletPath,
    aes_key: Vec<u8>,
    salt: Option<[u8; SALT_SIZE]>,
    iv: Option<[u8; IV_SIZE]>,
}

impl SecureWalletFile for WalletFile {
    fn path(&self) -> &WalletPath {
        &self.path
    }

    fn aes_key(&self) -> &[u8] {
        &self.aes_key
    }

    fn zeroize_aes_key(&mut self) {
        self.aes_key.zeroize();
    }

    fn salt(&self) -> Option<&[u8; SALT_SIZE]> {
        self.salt.as_ref()
    }

    fn iv(&self) -> Option<&[u8; IV_SIZE]> {
        self.iv.as_ref()
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = exec().await {
        // display the error message (if any)
        match err.downcast_ref::<InquireError>() {
            Some(
                InquireError::OperationInterrupted
                | InquireError::OperationCanceled,
            ) => (),
            _ => eprintln!("{err}"),
        };
        // give cursor back to the user
        io::prompt::show_cursor()?;
    }
    Ok(())
}

async fn connect<F>(
    mut wallet: Wallet<F>,
    settings: &Settings,
    status: fn(&str),
) -> anyhow::Result<Wallet<F>>
where
    F: SecureWalletFile + std::fmt::Debug,
{
    let con = wallet
        .connect_with_status(
            settings.state.as_str(),
            settings.prover.as_str(),
            settings.archiver.as_str(),
            status,
        )
        .await;

    // check for connection errors
    match con {
        Err(Error::RocksDB(e)) => {
            wallet.close();

            let msg = match e.kind() {
                ErrorKind::InvalidArgument => {
                    format!("You seem to try access a wallet with a different mnemonic phrase\n\r\n\r{0: <1} delete the cache? (Alternatively specify the --wallet-dir flag to add a new wallet under the given path)", "[ALERT]")
                },
                ErrorKind::Corruption => {
                       format!("The database appears to be corrupted \n\r\n\r{0: <1} delete the cache?", "[ALERT]")
                },
                _ => {
                    format!("Unknown database error {:?} \n\r\n\r{1: <1} delete the cache?", e, "[ALERT]")
                }
            };

             match prompt::ask_confirm_erase_cache(&msg)? {
                true => {
                    if let Some(io_err) = wallet.delete_cache().err() {
                        error!("Error while deleting the cache: {io_err}");
                    }

                    info!("Restart the application to create new wallet.");
                },
                false => {
                    info!("Wallet cannot proceed will now exit");
                },

            }

            return Err(anyhow::anyhow!("Wallet cannot proceed will now exit"));
        },
        Err(ref e) => warn!("[OFFLINE MODE]: Unable to connect to Rusk, limited functionality available: {e}"),
        _ => {}
    };

    Ok(wallet)
}

async fn exec() -> anyhow::Result<()> {
    // parse user args
    let args = WalletArgs::parse();
    // get the subcommand, if it is `None` we run the wallet in interactive mode
    let cmd = args.command.clone();

    // Get the initial settings from the args
    let mut settings_builder = Settings::args(args)?;

    // Obtain the wallet dir from the settings
    let wallet_dir = settings_builder.wallet_dir().clone();

    fs::create_dir_all(wallet_dir.as_path())
        .inspect_err(|_| settings_builder.args.password.zeroize())?;

    // prepare wallet path
    let mut wallet_path =
        WalletPath::from(wallet_dir.as_path().join("wallet.dat"));

    // load configuration (or use default)
    let cfg = Config::load(&wallet_dir)
        .inspect_err(|_| settings_builder.args.password.zeroize())?;

    wallet_path.set_network_name(settings_builder.args.network.clone());

    // Finally complete the settings by setting the network
    let mut settings = settings_builder
        .network(cfg.network)
        .map_err(|_| rusk_wallet::Error::NetworkNotFound)?;

    // generate a subscriber with the desired log level
    //
    // TODO: we should have the logger instantiate sooner, otherwise we cannot
    // catch errors that are happened before its instantiation.
    //
    // Therefore, the logger details such as `type` and `level` cannot be part
    // of the configuration, since it won't catch any configuration error
    // otherwise.
    //
    // See: <https://github.com/dusk-network/wallet-cli/issues/73>
    //
    let level = &settings.logging.level;
    let level: Level = level.into();
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(level)
        .with_writer(std::io::stderr);

    // set the subscriber as global
    match settings.logging.format {
        LogFormat::Json => {
            let subscriber = subscriber.json().flatten_event(true).finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        LogFormat::Plain => {
            let subscriber = subscriber.with_ansi(false).finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        LogFormat::Coloured => {
            let subscriber = subscriber.finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
    };

    let is_headless = cmd.is_some();

    if let Some(Command::Settings) = cmd {
        println!("{}", &settings);
        settings.password.zeroize();
        return Ok(());
    };

    // get our wallet ready
    let mut wallet: Wallet<WalletFile> =
        get_wallet(&cmd, &settings, &wallet_path)
            .await
            .inspect_err(|_| settings.password.zeroize())?;

    let file_version = wallet.get_file_version().inspect_err(|_| {
        wallet.close();
        settings.password.zeroize();
    })?;

    if file_version.is_old() {
        update_wallet_file(&mut wallet, &settings.password, file_version)
            .inspect_err(|_| {
                wallet.close();
                settings.password.zeroize();
            })?;
    }

    // set our status callback
    let status_cb = match is_headless {
        true => status::headless,
        false => status::interactive,
    };

    wallet = connect(wallet, &settings, status_cb)
        .await
        .inspect_err(|_| {
            settings.password.zeroize();
        })?;

    let res = run_command_or_enter_loop(&mut wallet, &settings, cmd).await;

    wallet.close();
    settings.password.zeroize();

    res?;

    Ok(())
}

async fn run_command_or_enter_loop(
    wallet: &mut Wallet<WalletFile>,
    settings: &Settings,
    cmd: Option<Command>,
) -> anyhow::Result<()> {
    // run command
    match cmd {
        // if there is no command we are in interactive mode and need to run the
        // interactive loop
        None => {
            wallet.register_sync()?;
            interactive::run_loop(wallet, settings).await?;
        }
        // else we run the given command and print the result
        Some(cmd) => {
            match cmd.run(wallet, settings).await? {
                RunResult::PhoenixBalance(balance, spendable) => {
                    if spendable {
                        println!("{}", Dusk::from(balance.spendable));
                    } else {
                        println!("{}", Dusk::from(balance.value));
                    }
                }
                RunResult::MoonlightBalance(balance) => {
                    println!("Total: {}", balance);
                }
                RunResult::Profile((profile_idx, profile)) => {
                    println!(
                        "> {}\n>   {}\n>   {}\n",
                        Profile::index_string(profile_idx),
                        profile.shielded_account_string(),
                        profile.public_account_string(),
                    );
                }
                RunResult::Profiles(addrs) => {
                    for (profile_idx, profile) in addrs.iter().enumerate() {
                        println!(
                            "> {}\n>   {}\n>   {}\n\n",
                            Profile::index_string(profile_idx as u8),
                            profile.shielded_account_string(),
                            profile.public_account_string(),
                        );
                    }
                }
                RunResult::Tx(hash) => {
                    let tx_id = hex::encode(hash.to_bytes());

                    // Wait for transaction confirmation from network
                    let gql = GraphQL::new(
                        settings.state.clone(),
                        settings.archiver.clone(),
                        status::headless,
                    )?;
                    gql.wait_for(&tx_id).await?;

                    println!("{tx_id}");
                }
                RunResult::DeployTx(hash, contract_id) => {
                    let tx_id = hex::encode(hash.to_bytes());
                    let contract_id = hex::encode(contract_id.as_bytes());
                    println!("Deploying {contract_id}",);

                    // Wait for transaction confirmation from network
                    let gql = GraphQL::new(
                        settings.state.clone(),
                        settings.archiver.clone(),
                        status::headless,
                    )?;
                    gql.wait_for(&tx_id).await?;

                    println!("{tx_id}");
                }
                RunResult::StakeInfo(info, reward) => {
                    let rewards = Dusk::from(info.reward);
                    if reward {
                        println!("{rewards}");
                    } else {
                        if let Some(amt) = info.amount {
                            let amount = Dusk::from(amt.value);
                            let locked = Dusk::from(amt.locked);
                            let eligibility = amt.eligibility;
                            let epoch = amt.eligibility / EPOCH;

                            println!("Eligible stake: {amount} DUSK");
                            println!(
                                "Reclaimable slashed stake: {locked} DUSK"
                            );
                            println!("Stake active from block #{eligibility} (Epoch {epoch})");
                        } else {
                            println!("No active stake found for this key");
                        }
                        let faults = info.faults;
                        let hard_faults = info.hard_faults;
                        let rewards = Dusk::from(info.reward);

                        println!("Slashes: {faults}");
                        println!("Hard Slashes: {hard_faults}");
                        println!("Accumulated rewards is: {rewards} DUSK");
                    }
                }
                RunResult::ExportedKeys(pub_key, key_pair) => {
                    println!("{},{}", pub_key.display(), key_pair.display())
                }
                RunResult::History(txns) => {
                    if let Err(err) = crate::prompt::tx_history_list(&txns) {
                        match err.downcast_ref::<InquireError>() {
                            Some(InquireError::OperationInterrupted | InquireError::OperationCanceled) => (),
                            _ => println!("Failed to output transaction history with error {err}"),
                        }
                    }
                }
                RunResult::ContractId(id) => {
                    println!("Contract ID: {:?}", id);
                }
                RunResult::Settings() => {}
                RunResult::Create() | RunResult::Restore() => {}
            }
        }
    };
    Ok(())
}

async fn get_wallet(
    cmd: &Option<Command>,
    settings: &Settings,
    wallet_path: &WalletPath,
) -> anyhow::Result<Wallet<WalletFile>> {
    let password = &settings.password;
    let wallet = match cmd {
        // if `cmd` is `None` we are in interactive mode and need to load the
        // wallet from file
        None => interactive::load_wallet(wallet_path, settings).await?,
        // else we check if we need to replace the wallet and then load it
        Some(ref cmd) => match cmd {
            Command::Create {
                skip_recovery,
                seed_file,
            } => Command::run_create(
                *skip_recovery,
                seed_file,
                password,
                wallet_path,
                &Prompter,
            )?,
            Command::Restore { file } => {
                match file {
                    Some(file) => {
                        // if we restore and old version file make sure we
                        // know the corrrect version before asking for the
                        // password
                        let (file_version, salt_and_iv) =
                            dat::read_file_version_and_salt_iv(file)?;

                        let mut key = prompt::derive_key_from_password(
                            "Please enter wallet password",
                            password,
                            salt_and_iv.map(|si| si.0).as_ref(),
                            file_version,
                        )?;

                        let mut w = Wallet::from_file(WalletFile {
                            path: file.clone(),
                            aes_key: key.clone(),
                            salt: salt_and_iv.map(|si| si.0),
                            iv: salt_and_iv.map(|si| si.1),
                        })
                        .inspect_err(|_| key.zeroize())?;

                        let (salt, iv) = salt_and_iv
                            .unwrap_or_else(|| (gen_salt(), gen_iv()));
                        w.save_to(WalletFile {
                            path: wallet_path.clone(),
                            aes_key: key,
                            salt: Some(salt),
                            iv: Some(iv),
                        })
                        .inspect_err(|_| w.close())?;
                        w
                    }
                    None => {
                        Command::run_restore_from_seed(wallet_path, &Prompter)?
                    }
                }
            }

            _ => {
                // Grab the file version for a random command
                let (file_version, salt_and_iv) =
                    dat::read_file_version_and_salt_iv(wallet_path)?;

                // load wallet from file
                let key = prompt::derive_key_from_password(
                    "Please enter wallet password",
                    password,
                    salt_and_iv.map(|si| si.0).as_ref(),
                    file_version,
                )?;

                Wallet::from_file(WalletFile {
                    path: wallet_path.clone(),
                    aes_key: key,
                    salt: salt_and_iv.map(|si| si.0),
                    iv: salt_and_iv.map(|si| si.1),
                })?
            }
        },
    };
    Ok(wallet)
}

fn update_wallet_file(
    wallet: &mut Wallet<WalletFile>,
    password: &Option<String>,
    file_version: DatFileVersion,
) -> Result<(), anyhow::Error> {
    let salt = gen_salt();
    let iv = gen_iv();
    let pwd = match password.as_ref() {
        Some(p) => p.to_string(),
        None => ask_pwd("Updating your wallet data file, please enter your wallet password ")?,
    };

    let old_wallet_file = wallet
        .file()
        .clone()
        .expect("wallet file should never be none");

    let old_key = derive_key(file_version, &pwd, old_wallet_file.salt())?;
    // Is the password correct?
    Wallet::from_file(WalletFile {
        aes_key: old_key,
        ..old_wallet_file.clone()
    })?;

    let old_wallet_path = save_old_wallet(&old_wallet_file.path)?;

    let key = derive_key(
        DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
        &pwd,
        Some(&salt),
    )?;
    wallet.save_to(WalletFile {
        path: old_wallet_file.path,
        aes_key: key,
        salt: Some(salt),
        iv: Some(iv),
    })?;
    println!(
        "Update successful. Old wallet data file is saved at {}",
        old_wallet_path.display()
    );

    Ok(())
}

fn save_old_wallet(wallet_path: &WalletPath) -> Result<PathBuf, Error> {
    let mut old_wallet_path = wallet_path.wallet.clone();
    old_wallet_path.pop();
    old_wallet_path.push("wallet.dat.old");
    fs::copy(&wallet_path.wallet, &old_wallet_path)?;
    Ok(old_wallet_path)
}
