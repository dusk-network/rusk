// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, read, remove_file, write, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

static CRS_17: &str =
    "caa176d248b24e6a324baf04c21a3c86a200767519cf5f823c68e3ab58cf9ef1";

#[derive(Debug, Clone)]
pub struct Keys([u8; 32]);

impl Keys {
    pub fn get_prover(&self) -> Result<Vec<u8>, io::Error> {
        let mut dir = get_rusk_keys_dir()?;
        dir.push(hex::encode(self.0));
        dir.set_extension("pk");

        match &dir.exists() {
            true => read(dir),
            false => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "ProverKey not found",
            )),
        }
    }

    pub fn get_verifier(&self) -> Result<Vec<u8>, io::Error> {
        let mut dir = get_rusk_keys_dir()?;
        dir.push(hex::encode(self.0));
        dir.set_extension("vd");

        match &dir.exists() {
            true => read(dir),
            false => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "VerifierData not found",
            )),
        }
    }
}

fn extension(p: &Path) -> Option<&str> {
    p.extension()?.to_str()
}

fn file_stem(p: &Path) -> Option<&str> {
    p.file_stem()?.to_str()
}

pub fn get_rusk_profile_dir() -> Result<PathBuf, io::Error> {
    env::var("RUSK_PROFILE_PATH")
        .map_or(home_dir(), |e| Some(PathBuf::from(e)))
        .and_then(|mut p| {
            p.push(".rusk");
            fs::create_dir_all(&p).map(|_| p).ok()
        })
        .ok_or_else(|| {
            warn!("rusk-profile dir not found and impossible to create");
            io::Error::new(
                io::ErrorKind::NotFound,
                "User Profile Dir not found",
            )
        })
}

fn get_rusk_keys_dir() -> Result<PathBuf, io::Error> {
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
            io::Error::new(
                io::ErrorKind::NotFound,
                "User Profile Dir not found",
            )
        })
}

pub fn get_rusk_state_dir() -> Result<PathBuf, io::Error> {
    env::var("RUSK_STATE_PATH")
        .map_or_else(
            |_| get_rusk_profile_dir().ok(),
            |e| Some(PathBuf::from(e)),
        )
        .and_then(|mut p| {
            p.push("state");
            fs::create_dir_all(&p).map(|_| p).ok()
        })
        .ok_or_else(|| {
            warn!("rusk-profile state dir not found and impossible to create");
            io::Error::new(io::ErrorKind::NotFound, "State Dir not found")
        })
}

pub fn to_rusk_state_id_path<P: AsRef<Path>>(dir: P) -> PathBuf {
    let dir = dir.as_ref();
    dir.join("state.id")
}

pub fn get_common_reference_string() -> Result<Vec<u8>, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    read(profile)
}

const CRS_FNAME: &str = "dev-piecrust.crs";

pub fn set_common_reference_string(buffer: Vec<u8>) -> Result<(), io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    write(&profile, buffer)?;
    info!("CRS added to cache");

    Ok(())
}

pub fn delete_common_reference_string() -> Result<(), io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push(CRS_FNAME);

    remove_file(&profile)?;
    info!("CRS removed from cache");

    Ok(())
}

pub fn verify_common_reference_string(buff: &[u8]) -> bool {
    info!("Checking integrity of CRS");
    let mut hasher = Sha256::new();
    hasher.update(buff);
    let hash = format!("{:x}", hasher.finalize());

    hash == CRS_17
}

pub fn clean_outdated_keys(ids: &[[u8; 32]]) -> Result<(), io::Error> {
    info!("Cleaning outdated keys (if any)");
    let ids_as_string: Vec<_> = ids.iter().map(hex::encode).collect();

    fs::read_dir(get_rusk_keys_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .filter(|p| match extension(p) {
            Some("pk" | "vd") => file_stem(p)
                .filter(|id| !ids_as_string.contains(&id.to_string()))
                .is_some(),
            _ => true,
        })
        .try_for_each(|p| {
            info!(
                "Found file {:?} which is not included in the keys list obtained",
                &p
            );
            remove_file(get_rusk_keys_dir()?.join(&p))?;
            info!("{:?} was successfully removed outdated file", &p);
            Ok(())
        })
}

pub fn keys_for(id: &[u8; 32]) -> Result<Keys, io::Error> {
    let mut dir = get_rusk_keys_dir()?;
    dir.push(hex::encode(id));

    // No need to check if the keys exist, because it's guaranteed to be
    // checked by the get_verifier and get_prover functions.
    Ok(Keys(*id))
}

pub fn add_keys_for(
    id: &[u8; 32],
    pk: Vec<u8>,
    vd: Vec<u8>,
) -> Result<(), io::Error> {
    let mut dir = get_rusk_keys_dir()?;
    dir.push(hex::encode(id));

    let pk_file = dir.with_extension("pk");
    let vd_file = dir.with_extension("vd");

    File::create(&pk_file)?.write_all(&pk)?;
    info!("Entry added: {:?}", pk_file);
    File::create(&vd_file)?.write_all(&vd)?;
    info!("Entry added: {:?}", vd_file);

    Ok(())
}

pub fn clear_all_keys() -> Result<(), io::Error> {
    info!("Clearing all the Keys folder contents");

    fs::read_dir(get_rusk_keys_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|res| res.ok())
        .filter(|e| e.is_file())
        .filter(|p| matches!(extension(p), Some("pk" | "vd")))
        .try_for_each(|path| {
            info!("Removing {:?}", path);
            remove_file(path)
        })
}
