// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use http_req::request;
use microkelvin::{BackendCtor, DiskBackend};
use rusk_vm::NetworkStateId;
use std::error::Error;
use std::io::{Cursor, Read};
use std::{fs, io};
use tracing::info;
use tracing::log::error;
use zip::ZipArchive;

#[cfg(feature = "genesis")]
pub use crate::genesis::deploy;

fn diskbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(|| {
        let dir = rusk_profile::get_rusk_state_dir()
            .expect("Failed to get Rusk profile directory");

        fs::remove_dir_all(&dir)
            .or_else(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .expect("Failed to clean up Network State directory");

        fs::create_dir_all(&dir)
            .expect("Failed to create Network State directory");

        DiskBackend::new(dir)
    })
}

#[cfg(not(feature = "genesis"))]
pub fn deploy<B>(
    _: bool,
    _: &BackendCtor<B>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + microkelvin::Backend,
{
    Err("No 'genesis' feature".into())
}

pub struct ExecConfig {
    pub build: bool,
    pub force: bool,
    pub testnet: bool,
}

pub fn exec(config: ExecConfig) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    info!("{} Network state", theme.action("Checking"));
    let state_path = rusk_profile::get_rusk_state_dir()?;
    let id_path = rusk_profile::get_rusk_state_id_path()?;

    // if we're not forcing a rebuild/download and the state already exists in
    // the expected path, stop early.
    if !config.force && state_path.exists() && id_path.exists() {
        info!("{} existing state", theme.info("Found"));

        let _ = NetworkStateId::read(&id_path)?;

        info!(
            "{} state id at {}",
            theme.success("Checked"),
            id_path.display()
        );
        return Ok(());
    }

    if config.build {
        info!("{} new state", theme.info("Building"));
        let state_id = deploy(config.testnet, &diskbackend())
            .expect("Failed to deploy network state");

        info!("{} persisted id", theme.success("Storing"));
        state_id.write(&id_path)?;
    } else {
        info!("{} state from previous build", theme.info("Downloading"));

        if let Err(err) = download_state() {
            error!("{} downloading state", theme.error("Failed"));
            return Err(err);
        }
    }

    if !state_path.exists() {
        error!(
            "{} network state at {}",
            theme.error("Missing"),
            state_path.display()
        );
        return Err("Missing state at expected path".into());
    }

    if !id_path.exists() {
        error!(
            "{} persisted id at {}",
            theme.error("Missing"),
            id_path.display()
        );
        return Err("Missing persisted id at expected path".into());
    }

    info!(
        "{} network state at {}",
        theme.success("Stored"),
        state_path.display()
    );
    info!(
        "{} persisted id at {}",
        theme.success("Stored"),
        id_path.display()
    );

    Ok(())
}

const STATE_URL: &str =
    "https://dusk-infra.ams3.digitaloceanspaces.com/keys/rusk-state.zip";

/// Downloads the state into the rusk profile directory.
fn download_state() -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    let mut buffer = vec![];
    let response = request::get(STATE_URL, &mut buffer)?;

    // only accept success codes.
    if !response.status_code().is_success() {
        return Err(format!(
            "State download error: HTTP {}",
            response.status_code()
        )
        .into());
    }

    info!("{} state archive into", theme.info("Unzipping"));

    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;

    let mut profile_path = rusk_profile::get_rusk_profile_dir()?;
    profile_path.pop();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_path = profile_path.join(entry.name());

        if entry.is_dir() {
            let _ = fs::create_dir_all(entry_path);
        } else {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            let _ = fs::write(entry_path, buffer)?;
        }
    }

    Ok(())
}
