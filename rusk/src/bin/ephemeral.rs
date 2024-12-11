// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::fs::File;
use std::io::{Read, Result};
use std::path::PathBuf;

use rusk_recovery_tools::state::tar;
use tempfile::TempDir;
use tracing::error;

pub(crate) fn configure(state_zip: &PathBuf) -> Result<Option<TempDir>> {
    let tmpdir = tempfile::tempdir()?;

    let state_dir = tmpdir.path().join("state");

    let mut f = File::open(state_zip)?;
    let mut data = Vec::new();
    f.read_to_end(&mut data)?;

    let unarchive = tar::unarchive(&data[..], state_dir.as_path());

    if let Err(e) = unarchive {
        error!("Invalid state input {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, ""));
    }

    env::set_var("RUSK_STATE_PATH", state_dir.as_os_str());

    Ok(Some(tmpdir))
}
