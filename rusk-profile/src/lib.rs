// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use std::env;
use std::fs::{self, read, remove_file, write};
use std::io;
use std::path::{Path, PathBuf};

mod theme;
pub use theme::Theme;

mod circuit;
pub use circuit::Circuit;

/// HEX representaion of the SHA-256 hash of the CRS uncompressed bytes.
pub static CRS_17_HASH: &str =
    "18b48f588fd4d1e88ef9e7b3cacfa29046f6f489c5c237a4b01ee4f0334772a5";

const CRS_FNAME: &str = "dev-piecrust.crs";

fn extension(p: &Path) -> Option<&str> {
    p.extension()?.to_str()
}

fn file_stem(p: &Path) -> Option<&str> {
    p.file_stem()?.to_str()
}

fn file_name(p: &Path) -> Option<&str> {
    p.file_name()?.to_str()
}

/// Return Rusk profile directory, ensuring that all parents directory are
/// created
///
/// Default to [`home_dir`]/.dusk/rusk
///
/// `RUSK_PROFILE_PATH` env can be used to override
///
/// E.g:
/// RUSK_PROFILE_PATH | result
/// -- | --
/// None | $HOME/.dusk/rusk
/// Set | $RUSK_PROFILE_PATH
pub fn get_rusk_profile_dir() -> io::Result<PathBuf> {
    env::var("RUSK_PROFILE_PATH")
        .map_or_else(
            |e| {
                home_dir()
                    .ok_or(io::Error::new(io::ErrorKind::InvalidInput, e))
                    .map(|p| p.join(".dusk").join("rusk"))
            },
            |profile_path| Ok(PathBuf::from(profile_path)),
        )
        .and_then(|p| fs::create_dir_all(&p).map(|_| p))
        .map_err(|e| {
            warn!("rusk-profile dir not found and impossible to create: {e}");
            e
        })
}

/// Return Rusk circuits directory, ensuring that all parents directory are
/// created
///
/// Default to [`get_rusk_profile_dir`]/circuits
///
/// `RUSK_CIRCUITS_PATH` env can be used to override
///
/// E.g:
/// RUSK_PROFILE_PATH | RUSK_CIRCUITS_PATH | result
/// -- | -- | --
/// None | None | $HOME/.dusk/rusk/circuits
/// Set | None | $RUSK_PROFILE_PATH/circuits
/// _ | Set | $RUSK_CIRCUITS_PATH
pub fn get_rusk_circuits_dir() -> io::Result<PathBuf> {
    env::var("RUSK_CIRCUITS_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().map(|p| p.join("circuits")),
            |circuits_path| Ok(PathBuf::from(circuits_path)),
        )
        .and_then(|p| fs::create_dir_all(&p).map(|_| p))
        .map_err(|e| {
            warn!("rusk-profile circuits dir not found and impossible to create: {e}");
            e
        })
}

/// Return Rusk keys directory, ensuring that all parents directory are created
///
/// Default to [`get_rusk_profile_dir`]/keys
///
/// `RUSK_KEYS_PATH` env can be used to override
///
/// E.g:
/// RUSK_PROFILE_PATH | RUSK_KEYS_PATH | result
/// -- | -- | --
/// None | None | $HOME/.dusk/rusk/keys
/// Set | None | $RUSK_PROFILE_PATH/keys
/// _ | Set | $RUSK_KEYS_PATH
pub fn get_rusk_keys_dir() -> io::Result<PathBuf> {
    env::var("RUSK_KEYS_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().map(|p| p.join("keys")),
            |keys_path| Ok(PathBuf::from(keys_path)),
        )
        .and_then(|p| fs::create_dir_all(&p).map(|_| p))
        .map_err(|e| {
            warn!("rusk-profile key's dir not found and impossible to create: {e}");
            e
        })
}

/// Return Rusk keys directory, ensuring that all parents directory are created
///
/// Default  to [get_rusk_profile_dir]/state
///
/// `RUSK_STATE_PATH` env can be used to override
///
/// E.g:
/// RUSK_PROFILE_PATH | RUSK_STATE_PATH | result
/// -- | -- | --
/// None | None | $HOME/.dusk/rusk/state
/// Set | None | $RUSK_PROFILE_PATH/state
/// _ | Set | $RUSK_STATE_PATH
pub fn get_rusk_state_dir() -> io::Result<PathBuf> {
    env::var("RUSK_STATE_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().map(|p| p.join("state")),
            |state_path| Ok(PathBuf::from(state_path)),
        )
        .and_then(|p| fs::create_dir_all(&p).map(|_| p))
        .map_err(|e| {
            warn!("rusk-profile state dir not found and impossible to create: {e}");
            e
        })
}

pub fn to_rusk_state_id_path<P: AsRef<Path>>(dir: P) -> PathBuf {
    let dir = dir.as_ref();
    dir.join("state.id")
}

pub fn to_rusk_epoch_id_path<P: AsRef<Path>>(dir: P) -> PathBuf {
    let dir = dir.as_ref();
    dir.join("epoch.id")
}

pub fn get_common_reference_string() -> io::Result<Vec<u8>> {
    let crs = get_rusk_profile_dir()?.join(CRS_FNAME);
    read(crs)
}

pub fn set_common_reference_string(buffer: Vec<u8>) -> io::Result<()> {
    if !verify_common_reference_string(&buffer[..]) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CRS Mismatch",
        ));
    }
    let crs = get_rusk_profile_dir()?.join(CRS_FNAME);
    write(crs, buffer)?;
    info!("{} CRS to cache", Theme::default().success("Added"),);

    Ok(())
}

pub fn delete_common_reference_string() -> io::Result<()> {
    let crs = get_rusk_profile_dir()?.join(CRS_FNAME);
    remove_file(crs)?;
    warn!("{}   CRS", Theme::default().warn("Removed"),);

    Ok(())
}

pub fn verify_common_reference_string(buff: &[u8]) -> bool {
    info!("{} CRS integrity", Theme::default().info("Checking"));
    let mut hasher = Sha256::new();
    hasher.update(buff);
    let hash = format!("{:x}", hasher.finalize());

    hash == CRS_17_HASH
}

pub fn clean_outdated(circuits: &[Circuit]) -> io::Result<()> {
    let ids_as_string: Vec<&str> =
        circuits.iter().map(|c| c.id_str()).collect();

    clean_outdated_circuits(&ids_as_string)?;
    clean_outdated_keys(&ids_as_string)
}

fn clean_outdated_circuits(ids: &[&str]) -> io::Result<()> {
    // removing all untracked files in circuits directory
    fs::read_dir(get_rusk_circuits_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .filter(|p| match extension(p) {
            Some("cd" | "toml") => {
                file_stem(p).filter(|id| ids.contains(id)).is_none()
            }
            _ => true,
        })
        .try_for_each(|p| {
            warn!(
                "{}   /circuits/{}",
                Theme::default().warn("Removing"),
                file_name(&p).expect("file should be valid")
            );
            remove_file(p)?;
            Ok(())
        })
}

fn clean_outdated_keys(ids: &[&str]) -> io::Result<()> {
    // removing all untracked files in keys directory
    fs::read_dir(get_rusk_keys_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .filter(|p| match extension(p) {
            Some("pk" | "vd") => {
                file_stem(p).filter(|id| ids.contains(id)).is_none()
            }
            _ => true,
        })
        .try_for_each(|p| {
            warn!(
                "{}   /keys/{}",
                Theme::default().warn("Removing"),
                file_name(&p).expect("file should be valid")
            );
            remove_file(p)
        })
}

pub fn clear_all() -> io::Result<()> {
    clear_all_circuits()?;
    clear_all_keys()
}

fn clear_all_keys() -> io::Result<()> {
    info!(
        "{} all keys directory contents",
        Theme::default().warn("Clearing")
    );

    fs::read_dir(get_rusk_keys_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .try_for_each(remove_file)
}

fn clear_all_circuits() -> io::Result<()> {
    info!(
        "{} all circuit directory contents",
        Theme::default().warn("Clearing")
    );

    fs::read_dir(get_rusk_circuits_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .try_for_each(remove_file)
}
