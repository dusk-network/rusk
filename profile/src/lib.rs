// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use sha2::{Digest, Sha256};
use std::fs::{self, read, write, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub struct Keys {
    crate_name: String,
    version: String,
}

impl Keys {
    pub fn get_dir(&self) -> Option<PathBuf> {
        if let Ok(mut dir) = get_rusk_keys_dir() {
            dir.push(&self.crate_name);
            dir.push(&self.version);
            Some(dir)
        } else {
            None
        }
    }

    pub fn are_outdated(&self) -> bool {
        self.get_dir().map_or(true, |dir| !dir.exists())
    }

    pub fn get(&self, handle: &str) -> Option<(Vec<u8>, Vec<u8>)> {
        let dir = self.get_dir();

        dir.filter(|dir| dir.exists()).and_then(|dir| {
            let mut hasher = Sha256::new();
            hasher.update(handle.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            let mut pk_file = dir.clone();
            pk_file.push(format!("{}.pk", hash));
            let mut vk_file = dir;
            vk_file.push(format!("{}.vk", hash));

            let pk = read(pk_file);
            let vk = read(vk_file);

            match (pk, vk) {
                (Ok(pk), Ok(vk)) => Some((pk, vk)),
                (_, _) => None,
            }
        })
    }

    pub fn get_prover(&self, handle: &str) -> Option<Vec<u8>> {
        let dir = self.get_dir();

        dir.filter(|dir| dir.exists()).and_then(|mut dir| {
            let mut hasher = Sha256::new();
            hasher.update(handle.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            dir.push(format!("{}.pk", hash));

            read(dir).ok()
        })
    }

    pub fn get_verifier(&self, handle: &str) -> Option<Vec<u8>> {
        let dir = self.get_dir();

        dir.filter(|dir| dir.exists()).and_then(|mut dir| {
            let mut hasher = Sha256::new();
            hasher.update(handle.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            dir.push(format!("{}.vk", hash));

            read(dir).ok()
        })
    }

    pub fn clear_all(&self) -> Result<(), io::Error> {
        let mut dir = get_rusk_keys_dir()?;

        dir.push(&self.crate_name);
        if dir.exists() {
            fs::remove_dir_all(dir.clone())?;
        }

        Ok(())
    }

    pub fn update(
        &self,
        handle: &str,
        keys: (Vec<u8>, Vec<u8>),
    ) -> Result<(), io::Error> {
        let mut dir = get_rusk_keys_dir()?;

        dir.push(&self.crate_name);
        dir.push(&self.version);

        fs::create_dir_all(dir.clone())?;

        let mut hasher = Sha256::new();
        hasher.update(handle.as_bytes());

        let hash = format!("{:x}", hasher.finalize());

        let mut pk_file = dir.clone();
        pk_file.push(format!("{}.pk", hash));

        let mut vk_file = dir;
        vk_file.push(format!("{}.vk", hash));

        File::create(pk_file)?.write_all(&keys.0)?;
        File::create(vk_file)?.write_all(&keys.1)?;

        Ok(())
    }
}

pub fn get_rusk_profile_dir() -> Result<PathBuf, io::Error> {
    if let Some(mut dir) = home_dir() {
        dir.push(".rusk");
        // PathBuf::from(BLINDBID_CIRCUIT_PK_PATH).exists(),
        fs::create_dir_all(dir.clone())?;
        Ok(dir)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "User Profile Dir not found",
        ))
    }
}

pub fn get_rusk_keys_dir() -> Result<PathBuf, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("keys");
    fs::create_dir_all(profile.clone())?;
    Ok(profile)
}

/// TODO: change name
pub fn get_common_reference_string() -> Result<Vec<u8>, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("dev.crs");

    Ok(read(profile)?)
}

/// TODO: change name
pub fn set_common_reference_string<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<u8>, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("dev.crs");

    let buff = read(&path)?;
    write(&profile, &buff)?;
    Ok(buff)
}

pub fn keys_for(crate_name: &str) -> Keys {
    use cargo_lock::{Lockfile, Package};
    let lockfile = Lockfile::load("./Cargo.lock").unwrap();

    let packages = lockfile
        .packages
        .iter()
        .filter(|package| crate_name == package.name.as_str())
        .collect::<Vec<&Package>>();

    // TODO: returns an error
    if packages.len() > 1 {
        panic!("Found {} version of {}", packages.len(), crate_name);
    }
    let package = packages[0];

    let version = format!("{}", package.version);

    Keys {
        crate_name: crate_name.to_string(),
        version,
    }
}
