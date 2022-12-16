// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod task;
mod version;

use clap::Parser;
use clap::builder::ArgAction;
use std::path::PathBuf;
use version::VERSION_BUILD;

#[derive(Parser, Debug)]
#[clap(name = "rusk-recovery-keys")]
#[clap(author, version = &VERSION_BUILD[..], about, long_about = None)]
struct Cli {
    /// Sets the profile path
    #[clap(
        short,
        long,
        value_parser,
        value_name = "PATH",
        env = "RUSK_PROFILE_PATH"
    )]
    profile: PathBuf,

    /// Keeps untracked keys
    #[clap(short, long, env = "RUSK_KEEP_KEYS")]
    keep: bool,

    /// Sets different levels of verbosity
    #[clap(short, long, action = ArgAction::Count)]
    verbose: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    task::run(
        || rusk_recovery_tools::keys::exec(args.keep),
        args.profile,
        args.verbose,
    )
}
