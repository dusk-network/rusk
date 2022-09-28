// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{value_parser, Arg, Command};
use std::{
    env,
    fs::{self, File},
    io::{BufReader, Read, Result},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::ZipArchive;

pub(crate) fn inject_args(command: Command<'_>) -> Command<'_> {
    command.arg(
        Arg::new("state_zip_file")
            .long("state")
            .short('s')
            .env("RUSK_STATE_ZIP_FILE")
            .help("Ephemeral state source path (.zip)")
            .takes_value(true)
            .value_parser(value_parser!(PathBuf))
            .required(false),
    )
}

pub(crate) fn configure(state_zip: &PathBuf) -> Result<Option<TempDir>> {
    let tmpdir = tempfile::tempdir()?;
    unzip(state_zip, tmpdir.path())?;

    env::set_var("RUSK_STATE_PATH", tmpdir.as_ref().as_os_str());

    Ok(Some(tmpdir))
}

/// Unzip a file into the output directory.
fn unzip(zipfile: &PathBuf, output: &Path) -> Result<()> {
    let f = File::open(zipfile)?;
    let reader = BufReader::new(f);
    let mut zip = ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_path = output.join(entry.name());

        if entry.is_dir() {
            let _ = fs::create_dir_all(entry_path);
        } else {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            fs::write(entry_path, buffer)?;
        }
    }

    Ok(())
}
