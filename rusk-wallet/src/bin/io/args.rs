// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::settings::{LogFormat, LogLevel};
use crate::Command;
use clap::{AppSettings, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(name = "Dusk Wallet CLI")]
#[clap(author = "Dusk Network B.V.")]
#[clap(about = "A user-friendly, reliable command line interface to the Dusk wallet!", long_about = None)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub(crate) struct WalletArgs {
    /// Directory to store user data [default: `$HOME/.dusk/rusk-wallet`]
    #[clap(short, long)]
    pub profile: Option<PathBuf>,

    /// Network to connect to
    #[clap(short, long)]
    pub network: Option<String>,

    /// Set the password for wallet's creation
    #[clap(long, env = "RUSK_WALLET_PWD")]
    pub password: Option<String>,

    /// The state server fully qualified URL
    #[clap(long)]
    pub state: Option<String>,

    /// The prover server fully qualified URL
    #[clap(long)]
    pub prover: Option<String>,

    /// Output log level
    #[clap(long, value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// Logging output type
    #[clap(long, value_enum, default_value_t = LogFormat::Coloured)]
    pub log_type: LogFormat,

    /// Command
    #[clap(subcommand)]
    pub command: Option<Command>,
}
