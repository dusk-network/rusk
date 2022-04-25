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

use rusk_recovery_tools::state::{exec, ExecConfig};

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

    /// Builds the state from scratch instead of downloading it.
    ///
    /// This flag requires the `genesis` feature.
    #[clap(short = 'w', long, env = "RUSK_BUILD_STATE")]
    build: bool,

    /// Forces a build/download even if the state is in the profile path.
    #[clap(short = 'f', long, env = "RUSK_FORCE_STATE")]
    force: bool,

    /// Builds a testnet state instead of a main. The state includes a
    /// transparent note with a billion Dusk, assigned to a hardcoded faucet
    /// address.
    ///
    /// If `build` is not set, this setting has no effect.
    #[clap(short = 't', long, env = "RUSK_BUILD_TESTNET")]
    testnet: bool,

    /// Sets different levels of verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    task::run(
        || {
            exec(ExecConfig {
                build: args.build,
                force: args.force,
                testnet: args.testnet,
            })
        },
        args.profile,
        args.verbose,
    )
}
