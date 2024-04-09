// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

use std::{env, fs, io};

use rusk_recovery_tools::state::{deploy, restore_state, tar};
use rusk_recovery_tools::Theme;
use tracing::info;

pub fn recovery_state(
    init: Option<PathBuf>,
    force: bool,
    output_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = match &init {
        Some(path) => fs::read_to_string(path)
            .map_err(|_| format!("file {path:?} not found"))?,
        None => rusk_recovery_tools::state::DEFAULT_SNAPSHOT.into(),
    };
    let init = toml::from_str(&config)?;

    let theme = Theme::default();
    info!("{} Network state", theme.action("Checking"));

    let _tmpdir = match output_file.clone() {
        Some(output) if output.exists() => Err("Output already exists")?,
        Some(_) => {
            let tmp_dir = tempfile::tempdir()?;
            env::set_var("RUSK_STATE_PATH", tmp_dir.path());
            Some(tmp_dir)
        }
        None => None,
    };

    if force {
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

    let (_, commit_id) = deploy(&state_dir, &init, |_| {})?;

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

    if let Some(output) = output_file {
        let state_folder = rusk_profile::get_rusk_state_dir()?;
        info!(
            "{} state into {}",
            theme.info("Compressing"),
            output.display()
        );
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
