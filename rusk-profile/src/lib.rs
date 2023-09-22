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
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

mod theme;
pub use theme::Theme;

mod circuit;
pub use circuit::Circuit;

static CRS_17: &str =
    "314cb1b373350a1c139b249bf55ae733884759cf968529d963c5bd4a7a2ef7c4";

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

pub fn get_rusk_profile_dir() -> io::Result<PathBuf> {
    env::var("RUSK_PROFILE_PATH")
        .map_or(home_dir(), |e| Some(PathBuf::from(e)))
        .and_then(|mut p| {
            p.push(".rusk");
            fs::create_dir_all(&p).map(|_| p).ok()
        })
        .ok_or_else(|| {
            warn!("rusk-profile dir not found and impossible to create");
            io::Error::new(ErrorKind::NotFound, "User Profile Dir not found")
        })
}

fn get_rusk_circuits_dir() -> io::Result<PathBuf> {
    env::var("RUSK_CIRCUITS_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().ok(),
            |e| Some(PathBuf::from(e)),
        )
        .and_then(|mut p| {
            p.push("circuits");
            fs::create_dir_all(&p).map(|_| p).ok()
        })
        .ok_or_else(|| {
            warn!(
                "rusk-profile circuits dir not found and impossible to create"
            );
            io::Error::new(ErrorKind::NotFound, "Circuits Dir not found")
        })
}

fn get_rusk_keys_dir() -> io::Result<PathBuf> {
    env::var("RUSK_KEYS_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().ok(),
            |e| Some(PathBuf::from(e)),
        )
        .and_then(|mut p| {
            p.push("keys");
            fs::create_dir_all(&p).map(|_| p).ok()
        })
        .ok_or_else(|| {
            warn!("rusk-profile key's dir not found and impossible to create");
            io::Error::new(ErrorKind::NotFound, "User Profile Dir not found")
        })
}

pub fn get_rusk_state_dir() -> io::Result<PathBuf> {
    env::var("RUSK_STATE_PATH")
        .map_or_else(
            |_| {
                get_rusk_profile_dir().ok().map(|mut p| {
                    p.push("state");
                    p
                })
            },
            |e| Some(PathBuf::from(e)),
        )
        .and_then(|p| fs::create_dir_all(&p).map(|_| p).ok())
        .ok_or_else(|| {
            warn!("rusk-profile state dir not found and impossible to create");
            io::Error::new(ErrorKind::NotFound, "State Dir not found")
        })
}

pub fn to_rusk_state_id_path<P: AsRef<Path>>(dir: P) -> PathBuf {
    let dir = dir.as_ref();
    dir.join("state.id")
}

pub fn get_common_reference_string() -> io::Result<Vec<u8>> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    read(profile)
}

pub fn set_common_reference_string(buffer: Vec<u8>) -> io::Result<()> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    write(&profile, buffer)?;
    info!("{} CRS to cache", Theme::default().success("Added"),);

    Ok(())
}

pub fn delete_common_reference_string() -> io::Result<()> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    remove_file(&profile)?;
    warn!("{} CRS", Theme::default().warn("Removed"),);

    Ok(())
}

pub fn verify_common_reference_string(buff: &[u8]) -> bool {
    info!("{} CRS integrity", Theme::default().info("Checking"));
    let mut hasher = Sha256::new();
    hasher.update(buff);
    let hash = format!("{:x}", hasher.finalize());

    hash == CRS_17
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
