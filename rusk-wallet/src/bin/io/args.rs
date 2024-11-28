// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::settings::{LogFormat, LogLevel};
use crate::Command;
use clap::{arg, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "A user-friendly, reliable command-line interface to interact with the Dusk blockchain.",
    author = "Dusk Network B.V."
)]
pub(crate) struct WalletArgs {
    /// Directory to store user data [default: `$HOME/.dusk/rusk-wallet`]
    #[arg(short, long)]
    pub wallet_dir: Option<PathBuf>,

    /// Network to connect to
    #[arg(short, long)]
    pub network: Option<String>,

    /// Set the password for wallet's creation
    #[arg(long, env = "RUSK_WALLET_PWD")]
    pub password: Option<String>,

    /// The state server fully qualified URL
    #[arg(long)]
    pub state: Option<String>,

    /// The prover server fully qualified URL
    #[arg(long)]
    pub prover: Option<String>,

    /// Output log level
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// Logging output type
    #[arg(long, value_enum, default_value_t = LogFormat::Coloured)]
    pub log_type: LogFormat,

    /// format of the result messages written to stdout after a wallet
    /// operation
    #[clap(long, value_enum, default_value_t = LogFormat::Plain)]
    pub stdout_format: LogFormat,

    /// Command
    #[command(subcommand)]
    pub command: Option<Command>,
}
