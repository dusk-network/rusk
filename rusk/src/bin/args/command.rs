// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use super::state::recovery_state;
use clap::builder::BoolishValueParser;
use clap::Subcommand;
use rusk_recovery_tools::Theme;
use std::io;
use std::time::Instant;
use tracing::info;

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub enum Command {
    RecoveryKeys {
        /// Keeps untracked keys
        #[clap(short, long, value_parser = BoolishValueParser::new(), env = "RUSK_KEEP_KEYS")]
        keep: bool,
    },

    RecoveryState {
        /// Forces a build/download even if the state is in the profile path.
        #[clap(short = 'f', value_parser = BoolishValueParser::new(), long, env = "RUSK_FORCE_STATE")]
        force: bool,

        /// Create a state applying the init config specified in this file.
        #[clap(short, long, value_parser, env = "RUSK_RECOVERY_INPUT")]
        init: Option<PathBuf>,

        /// If specified, the generated state is written on this file instead
        /// of save the state in the profile path.
        #[clap(short, long, value_parser, num_args(1))]
        output: Option<PathBuf>,
    },
}

impl Command {
    fn display_env(theme: &Theme) -> io::Result<()> {
        let profile_dir = rusk_profile::get_rusk_profile_dir()?;
        let circuits_dir = rusk_profile::get_rusk_circuits_dir()?;
        let keys_dir = rusk_profile::get_rusk_keys_dir()?;
        let state_dir = rusk_profile::get_rusk_state_dir()?;

        info!("{} {}", theme.info("PROFILE"), profile_dir.display());
        info!("{} {}", theme.info("CIRCUITS"), circuits_dir.display());
        info!("{} {}", theme.info("KEYS"), keys_dir.display());
        info!("{} {}", theme.info("STATE"), state_dir.display());
        Ok(())
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let begin = Instant::now();

        let theme = Theme::default();

        Self::display_env(&theme)?;

        let result = match self {
            Self::RecoveryState {
                force,
                init,
                output,
            } => recovery_state(init, force, output),
            Self::RecoveryKeys { keep } => {
                rusk_recovery_tools::keys::exec(keep)
            }
        };

        if let Err(e) = &result {
            tracing::error!("{} {e}", theme.error("Error"));
        }

        let finished = theme.action("Finished");
        let elapsed = begin.elapsed().as_secs_f32();
        info!("{finished} task in {elapsed:.2}s",);

        result
    }
}
