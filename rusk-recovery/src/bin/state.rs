// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod task;
mod version;

use clap::Parser;
use std::path::PathBuf;
use version::VERSION_BUILD;

#[derive(Parser, Debug)]
#[clap(name = "rusk-recovery-state")]
#[clap(author, version = &VERSION_BUILD[..], about, long_about = None)]
struct Cli {
    /// Sets the profile path
    #[clap(
        short,
        long,
        parse(from_os_str),
        value_name = "PATH",
        env = "RUSK_PROFILE_PATH"
    )]
    profile: PathBuf,

    /// Overwrite the current state if exists
    #[clap(short = 'w', long, env = "RUSK_OVERWRITE_STATE")]
    overwrite: bool,

    /// Sets different levels of verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    task::run(
        || rusk_recovery_tools::state::exec(args.overwrite),
        args.profile,
        args.verbose,
    )
}
