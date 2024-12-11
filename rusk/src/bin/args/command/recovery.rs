// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;

use clap::builder::BoolishValueParser;
use clap::Subcommand;
use rusk_recovery_tools::Theme;
use tracing::info;

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub enum RecoveryCommand {
    #[cfg(feature = "recovery-keys")]
    /// Check ZK keys and regenerate them if missing
    Keys {
        /// Keeps untracked keys
        #[clap(short, long, value_parser = BoolishValueParser::new(), env = "RUSK_KEEP_KEYS")]
        keep: bool,

        /// URL of the server to download the CRS from
        #[clap(
            long,
            default_value = "https://testnet.nodes.dusk.network/trusted-setup",
            env = "RUSK_CRS_URL"
        )]
        crs_url: String,
    },

    #[cfg(feature = "recovery-state")]
    /// Check VM state and create a new one if missing
    State {
        /// Forces a build/download even if the state is in the profile path.
        #[clap(short = 'f', value_parser = BoolishValueParser::new(), long, env = "RUSK_FORCE_STATE")]
        force: bool,

        /// Create a state applying the init config specified in this file.
        #[clap(short, long, value_parser, env = "RUSK_RECOVERY_INPUT")]
        init: Option<std::path::PathBuf>,

        /// If specified, the generated state is written on this file instead
        /// of save the state in the profile path.
        #[clap(short, long, value_parser, num_args(1))]
        output: Option<std::path::PathBuf>,
    },
}

impl RecoveryCommand {
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
        let theme = Theme::default();

        Self::display_env(&theme)?;

        let result = match self {
            #[cfg(feature = "recovery-state")]
            Self::State {
                force,
                init,
                output,
            } => crate::args::state::recovery_state(init, force, output),
            #[cfg(feature = "recovery-keys")]
            Self::Keys { keep, crs_url } => {
                rusk_recovery_tools::keys::exec(keep, crs_url)
            }
        };

        if let Err(e) = &result {
            tracing::error!("{} {e}", theme.error("Error"));
        }

        result
    }
}
