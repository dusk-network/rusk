// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_recovery_tools::theme::Theme;
use std::env;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;

pub fn run(
    task: impl Fn() -> Result<(), Box<dyn std::error::Error>>,
    profile: PathBuf,
    verbose: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();

    env::set_var("RUSK_PROFILE_PATH", profile.to_str().unwrap());

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

    //rusk_recovery_tools::keys::exec(args.keep)?;
    task()?;

    info!(
        "{} task in {:.2}s",
        theme.action("Finished"),
        begin.elapsed().as_secs_f32()
    );
    Ok(())
}
