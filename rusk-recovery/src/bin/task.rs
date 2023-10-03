// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_recovery_tools::Theme;
use std::env;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::prelude::*;

pub fn run(
    task: impl Fn() -> Result<(), Box<dyn std::error::Error>>,
    profile: Option<PathBuf>,
    verbose: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();

    if let Some(profile) = profile {
        env::set_var("RUSK_PROFILE_PATH", profile.to_str().unwrap());
    }

    if verbose > 0 {
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
            .compact();

        tracing_subscriber::registry().with(fmt_layer).init();
    }

    let theme = Theme::default();
    info!(
        "{} {} as profile path",
        theme.action("Using"),
        rusk_profile::get_rusk_profile_dir()?.to_str().unwrap()
    );

    info!(
        "{} {} as circuits path",
        theme.action("Using"),
        rusk_profile::get_rusk_circuits_dir()?.to_str().unwrap()
    );

    info!(
        "{} {} as keys path",
        theme.action("Using"),
        rusk_profile::get_rusk_keys_dir()?.to_str().unwrap()
    );

    info!(
        "{} {} as state path",
        theme.action("Using"),
        rusk_profile::get_rusk_state_dir()?.to_str().unwrap()
    );

    task()?;

    info!(
        "{} task in {:.2}s",
        theme.action("Finished"),
        begin.elapsed().as_secs_f32()
    );
    Ok(())
}
