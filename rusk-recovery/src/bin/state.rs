// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod task;
mod version;

use clap::Parser;
use std::{fs, path::PathBuf};
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
    #[clap(short = 'w', long, env = "RUSK_BUILD_STATE")]
    build: bool,

    /// Forces a build/download even if the state is in the profile path.
    #[clap(short = 'f', long, env = "RUSK_FORCE_STATE")]
    force: bool,

    /// Create a state applying the init config specified in this file.
    #[clap(short, long, parse(from_os_str))]
    init: PathBuf,

    /// Sets different levels of verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Use prebuilt contracts when building the state from scratch.
    #[clap(short = 'c', long = "contracts", env = "RUSK_PREBUILT_CONTRACTS")]
    use_prebuilt_contracts: bool,

    /// If specified, the generated state is written on this file instead of
    /// save the state in the profile path.
    #[clap(short, long, parse(from_os_str), takes_value(true))]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let snapshot = fs::read_to_string(args.init)?;
    let snapshot = toml::from_str(&snapshot)?;

    task::run(
        || {
            exec(
                ExecConfig {
                    build: args.build,
                    force: args.force,
                    output_file: args.output.clone(),
                    use_prebuilt_contracts: args.use_prebuilt_contracts,
                },
                &snapshot,
            )
        },
        args.profile,
        args.verbose,
    )
}
