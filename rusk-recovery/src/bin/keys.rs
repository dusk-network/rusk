// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod task;
mod version;

use clap::Parser;
use version::VERSION_BUILD;

#[derive(Parser, Debug)]
#[clap(name = "rusk-recovery-keys")]
#[clap(author, version = &VERSION_BUILD[..], about, long_about = None)]
struct Cli {
    /// Keeps untracked keys
    #[clap(short, long, env = "RUSK_KEEP_KEYS")]
    keep: bool,

    /// Sets different levels of verbosity
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    task::run(
        || Ok(rusk_recovery_tools::keys::exec(args.keep)?),
        args.verbose,
    )
}
