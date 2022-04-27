// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use lazy_static::lazy_static;
use rusk_vm::NetworkStateId;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use tracing::info;
use tracing::log::error;
use zip::ZipArchive;

lazy_static! {
    pub static ref DUSK_KEY: PublicSpendKey = {
        let bytes = include_bytes!("../dusk.psk");
        PublicSpendKey::from_bytes(bytes)
            .expect("faucet should have a valid key")
    };
    pub static ref FAUCET_KEY: PublicSpendKey = {
        let bytes = include_bytes!("../faucet.psk");
        PublicSpendKey::from_bytes(bytes)
            .expect("faucet should have a valid key")
    };
}

pub struct ExecConfig {
    pub force: bool,
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

    info!("{} state from previous build", theme.info("Deploying"));
    
    let mut profile_path = rusk_profile::get_rusk_profile_dir()?;
    profile_path.pop();

    if let Err(err) = deploy_state(&profile_path) {
        error!("{} deploying state", theme.error("Failed"));
        return Err(err);
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

/// Deploy the state into a directory.
pub fn deploy_state(profile_path: &Path) -> Result<NetworkStateId, Box<dyn Error>> {
    let theme = Theme::default();

    let buffer = STATE_ZIP;

    info!("{} state archive into", theme.info("Unzipping"));

    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;

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
    let mut id_path = std::path::PathBuf::from(profile_path.clone());
    id_path.push("state.id");
    let state_id = NetworkStateId::read(&id_path)?;
    Ok(state_id)
}

// if state features is enable this constant include bytes from folder specified
// in OUT_DIR env variable otherwise it is empty
#[cfg(feature = "state")]
pub const STATE_ZIP: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/state.zip"));
