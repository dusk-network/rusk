// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod args;
mod version;

use rusk_recovery_tools::theme::Theme;
use std::env;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();
    let matches = args::matches();
    let profile = matches.value_of("profile").unwrap();

    env::set_var("RUSK_PROFILE_PATH", profile);

    if let Some((command, matches)) = matches.subcommand() {
        let is_verbose = matches.occurrences_of("verbose") > 0;

        if is_verbose {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .compact();

            tracing_subscriber::registry().with(fmt_layer).init();
        } else {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false)
                .with_level(false)
                .compact()
                .with_filter(filter_fn(|meta| {
                    meta.target().contains("rusk_recovery")
                }));

            tracing_subscriber::registry().with(fmt_layer).init();
        }

        let theme = Theme::default();
        info!(
            "{} {} as profile path",
            theme.action("Using"),
            rusk_profile::get_rusk_profile_dir()?.to_str().unwrap()
        );

        match command {
            "keys" => {
                rusk_recovery_tools::keys::exec(matches.is_present("keep"))?
            }
            "state" => {
                rusk_recovery_tools::state::exec(
                    matches.is_present("overwrite"),
                )?;
            }
            "reset" => {
                rusk_recovery_tools::keys::exec(matches.is_present("keep"))?;
                rusk_recovery_tools::state::exec(true)?;
            }
            _ => unreachable!(),
        };
        info!(
            "{} task(s) in {:.2}s",
            theme.action("Finished"),
            begin.elapsed().as_secs_f32()
        );
        Ok(())
    } else {
        unreachable!();
    }
}
