// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::version;
use clap::{arg, crate_description, App, AppSettings, ArgMatches};
use rustc_tools_util::VersionInfo;

pub fn matches() -> ArgMatches {
    let info = rustc_tools_util::get_version_info!();

    App::new(&info.crate_name)
    .version(&*version::version(&info))
    .about(crate_description!())
    .global_setting(AppSettings::PropagateVersion)
    .global_setting(AppSettings::UseLongFormatForHelpSubcommand)
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .subcommand(
      App::new("keys")
        .about("Generate circuits keys")
        .arg(arg!(-k - -keep "Keep unused keys").env("RUSK_KEEP_KEYS")),
    )
    .subcommand(
      App::new("state")
        .about("Generate a network state with Genesis Contracts deployed")
        .arg(
          arg!(-w - -overwrite "Overwrite the current state if exists")
            .env("RUSK_OVERWRITE_STATE"),
        ),
    )
    .subcommand(
      App::new("reset")
        .about(
          format!(
            "Reset to factory settings. Equivalent to run both: \n\
            \t{} state --profile [PATH] -w\n\
            \t{} --profile [PATH] keys",
            &info.crate_name, &info.crate_name
          )
          .as_str(),
        )
        .arg(arg!(-k - -keep "Keep unused keys").env("RUST_KEEP_KEYS")),
    )
    .arg(
      arg!(-p - -profile <PATH> "Profile directory").env("RUSK_PROFILE_PATH"),
    )
    .arg(
      arg!(-v - -verbose "Verbose output")
        .global(true)
        .multiple_occurrences(true),
    )
    .get_matches()
}
