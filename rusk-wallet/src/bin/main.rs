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

pub(crate) use command::{Command, RunResult};

use bip39::{Language, Mnemonic, MnemonicType};
use clap::Parser;
use inquire::InquireError;
use rocksdb::ErrorKind;
use tracing::{error, info, warn, Level};

use crate::settings::{LogFormat, Settings};

use rusk_wallet::{
    dat::{self, LATEST_VERSION},
    Error, GraphQL, Profile, SecureWalletFile, Wallet, WalletPath, EPOCH,
};

use config::Config;
use io::{prompt, status, WalletArgs};

use std::fs::{self, File};
use std::io::Write;

#[derive(Debug, Clone)]
pub(crate) struct WalletFile {
    path: WalletPath,
    pwd: Vec<u8>,
}

impl SecureWalletFile for WalletFile {
    fn path(&self) -> &WalletPath {
        &self.path
    }

    fn pwd(&self) -> &[u8] {
        &self.pwd
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = exec().await {
        // display the error message (if any)
        match err.downcast_ref::<InquireError>() {
            Some(InquireError::OperationInterrupted) => {
                // TODO: Handle this error properly
                // See also https://github.com/dusk-network/wallet-cli/issues/104
            }
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

            // Exit because we cannot proceed because of db error
            // wallet is already closed
            std::process::exit(1);
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
    let settings_builder = Settings::args(args)?;

    // Obtain the wallet dir from the settings
    let wallet_dir = settings_builder.wallet_dir().clone();

    fs::create_dir_all(wallet_dir.as_path())?;

    // prepare wallet path
    let mut wallet_path =
        WalletPath::from(wallet_dir.as_path().join("wallet.dat"));

    // load configuration (or use default)
    let cfg = Config::load(&wallet_dir)?;

    wallet_path.set_network_name(settings_builder.args.network.clone());

    // Finally complete the settings by setting the network
    let settings = settings_builder
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

    let password = &settings.password;

    if let Some(Command::Settings) = cmd {
        println!("{}", &settings);
        return Ok(());
    };

    let file_version = dat::read_file_version(&wallet_path);

    // get our wallet ready
    let mut wallet: Wallet<WalletFile> = match cmd {
        // if `cmd` is `None` we are in interactive mode and need to load the
        // wallet from file
        None => {
            interactive::load_wallet(&wallet_path, &settings, file_version)
                .await?
        }
        // else we check if we need to replace the wallet and then load it
        Some(ref cmd) => match cmd {
            Command::Create {
                skip_recovery,
                seed_file,
            } => {
                // create a new randomly generated mnemonic phrase
                let mnemonic =
                    Mnemonic::new(MnemonicType::Words12, Language::English);
                // ask user for a password to secure the wallet
                // latest version is used for dat file
                let pwd = prompt::create_password(
                    password,
                    dat::DatFileVersion::RuskBinaryFileFormat(LATEST_VERSION),
                )?;

                match (skip_recovery, seed_file) {
                    (_, Some(file)) => {
                        let mut file = File::create(file)?;
                        file.write_all(mnemonic.phrase().as_bytes())?
                    }
                    // skip phrase confirmation if explicitly
                    (false, _) => prompt::confirm_mnemonic_phrase(&mnemonic)?,
                    _ => {}
                }

                // create wallet
                let mut w = Wallet::new(mnemonic)?;

                w.save_to(WalletFile {
                    path: wallet_path,
                    pwd,
                })?;

                w
            }
            Command::Restore { file } => {
                let (mut w, pwd) = match file {
                    Some(file) => {
                        // if we restore and old version file make sure we
                        // know the corrrect version before asking for the
                        // password
                        let file_version = dat::read_file_version(file)?;

                        let pwd = prompt::request_auth(
                            "Please enter wallet password",
                            password,
                            file_version,
                        )?;

                        let w = Wallet::from_file(WalletFile {
                            path: file.clone(),
                            pwd: pwd.clone(),
                        })?;

                        (w, pwd)
                    }
                    // Use the latest dat file version when there's no dat file
                    // provided when restoring the wallet
                    None => {
                        // ask user for 12-word mnemonic phrase
                        let phrase = prompt::request_mnemonic_phrase()?;
                        // ask user for a password to secure the wallet
                        let pwd = prompt::create_password(
                            password,
                            dat::DatFileVersion::RuskBinaryFileFormat(
                                LATEST_VERSION,
                            ),
                        )?;
                        // create wallet
                        let w = Wallet::new(phrase)?;

                        (w, pwd)
                    }
                };

                w.save_to(WalletFile {
                    path: wallet_path,
                    pwd,
                })?;

                w
            }

            _ => {
                // Grab the file version for a random command
                let file_version = file_version?;
                // load wallet from file
                let pwd = prompt::request_auth(
                    "Please enter wallet password",
                    password,
                    file_version,
                )?;

                Wallet::from_file(WalletFile {
                    path: wallet_path,
                    pwd,
                })?
            }
        },
    };

    // set our status callback
    let status_cb = match is_headless {
        true => status::headless,
        false => status::interactive,
    };

    wallet = connect(wallet, &settings, status_cb).await?;

    // run command
    match cmd {
        // if there is no command we are in interactive mode and need to run the
        // interactive loop
        Some(cmd) => {
            let run_res = cmd.run(&mut wallet, &settings).await?;

            let mut stdout_res = run_res.to_string();

            if want_json {
                stdout_res = run_res.to_json();
            }

            println!("{stdout_res}");
        }
        None => {
            wallet.register_sync().await?;
            interactive::run_loop(&mut wallet, &settings).await?;
        }
    }

    wallet.close();

    Ok(())
}
