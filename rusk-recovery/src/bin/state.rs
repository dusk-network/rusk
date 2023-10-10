// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod task;
mod version;

use clap::builder::{ArgAction, BoolishValueParser};
use clap::Parser;
use rusk_recovery_tools::Theme;
use std::error::Error;
use std::{env, io};
use std::{fs, path::PathBuf};
use tracing::info;
use version::VERSION_BUILD;

use rusk_recovery_tools::state::{deploy, restore_state, tar, Snapshot};

#[derive(Parser, Debug)]
#[clap(name = "rusk-recovery-state")]
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
    profile: Option<PathBuf>,

    /// Forces a build/download even if the state is in the profile path.
    #[clap(short = 'f', value_parser = BoolishValueParser::new(), long, env = "RUSK_FORCE_STATE")]
    force: bool,

    /// Create a state applying the init config specified in this file.
    #[clap(short, long, value_parser, env = "RUSK_RECOVERY_INPUT")]
    init: Option<PathBuf>,

    /// Sets different levels of verbosity
    #[clap(short, long, action = ArgAction::Count)]
    verbose: usize,

    /// If specified, the generated state is written on this file instead of
    /// save the state in the profile path.
    #[clap(short, long, value_parser, num_args(1))]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let config = match args.init {
        Some(path) => fs::read_to_string(path)?,
        None => include_str!("../../config/testnet_remote.toml").into(),
    };
    let snapshot = toml::from_str(&config)?;

    task::run(
        || {
            exec(ExecConfig {
                init: &snapshot,
                force: args.force,
                output_file: args.output.clone(),
            })
        },
        args.profile,
        args.verbose,
    )
}

pub struct ExecConfig<'a> {
    pub init: &'a Snapshot,
    pub force: bool,
    pub output_file: Option<PathBuf>,
}

pub fn exec(config: ExecConfig) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();
    info!("{} Network state", theme.action("Checking"));

    let _tmpdir = match config.output_file.clone() {
        Some(output) if output.exists() => Err("Output already exists")?,
        Some(_) => {
            let tmp_dir = tempfile::tempdir()?;
            env::set_var("RUSK_STATE_PATH", tmp_dir.path());
            Some(tmp_dir)
        }
        None => None,
    };

    if config.force {
        clean_state()?;
    }

    let state_dir = rusk_profile::get_rusk_state_dir()?;
    let state_id_path = rusk_profile::to_rusk_state_id_path(&state_dir);

    let _ = rusk_abi::new_vm(&state_dir)?;

    // if the state already exists in the expected path, stop early.
    if state_dir.exists() && state_id_path.exists() {
        info!("{} existing state", theme.info("Found"));

        let (_, commit_id) = restore_state(state_dir)?;
        info!(
            "{} state id at {}",
            theme.success("Checked"),
            state_id_path.display()
        );
        info!("{} {}", theme.action("Root"), hex::encode(commit_id));

        return Ok(());
    }

    info!("{} new state", theme.info("Building"));

    let (_, commit_id) = deploy(&state_dir, config.init)?;

    info!("{} {}", theme.action("Final Root"), hex::encode(commit_id));

    info!(
        "{} network state at {}",
        theme.success("Stored"),
        state_dir.display()
    );
    info!(
        "{} persisted id at {}",
        theme.success("Stored"),
        state_id_path.display()
    );

    if let Some(output) = config.output_file {
        let state_folder = rusk_profile::get_rusk_state_dir()?;
        info!("{} state into the output file", theme.info("Zipping"),);
        tar::archive(&state_folder, &output)?;
    }

    Ok(())
}

fn clean_state() -> Result<(), io::Error> {
    let state_path = rusk_profile::get_rusk_state_dir()?;

    fs::remove_dir_all(state_path).or_else(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(e)
        }
    })
}
