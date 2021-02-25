// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use sha2::{Digest, Sha256};
use std::fs::{self, read, remove_file, write, File};
use std::io::prelude::*;
use std::path::PathBuf;
use std::{env, io};
use tracing::info;

static CRS_17: &str =
    "e1ebe5dedabf87d8fe1232e04d18a111530edc0f4beeeb0251d545a123d944fe";

pub struct Keys {
    crate_name: String,
    version: String,
}

impl Keys {
    pub fn get_dir(&self) -> Option<PathBuf> {
        if let Ok(mut dir) = get_rusk_keys_dir() {
            info!("Found the rusk keys dir");

            dir.push(&self.crate_name);
            dir.push(&self.version);
            Some(dir)
        } else {
            info!("Couldn't found the rusk keys dir");

            None
        }
    }

    pub fn are_outdated(&self) -> bool {
        let outdated = self.get_dir().map_or(true, |dir| !dir.exists());
        info!("keys outdated: {}", outdated);
        outdated
    }

    pub fn get(&self, handle: &str) -> Option<(Vec<u8>, Vec<u8>)> {
        info!("Getting VK and PK for handle: {}", handle);
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
                (Ok(pk), Ok(vk)) => {
                    info!("Found VK and PK in the cache");
                    Some((pk, vk))
                }
                (_, _) => {
                    info!("No VK and PK are present in the cache");
                    None
                }
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
        info!(
            "Clearing all the keys for any version of {}",
            &self.crate_name
        );
        let mut dir = get_rusk_keys_dir()?;

        dir.push(&self.crate_name);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
            info!("Keys removed from the cache");
        } else {
            info!("Noop, the folder didn't exist in the cache");
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

        info!(
            "Cache updated for VK and PK of {} {}, \"{}\"",
            &self.crate_name, &self.version, handle
        );

        Ok(())
    }
}

pub fn get_rusk_profile_dir() -> Result<PathBuf, io::Error> {
    if let Some(mut dir) = home_dir() {
        dir.push(".rusk");
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

pub fn get_common_reference_string() -> Result<Vec<u8>, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("dev.crs");

    read(profile)
}

pub fn set_common_reference_string(buffer: Vec<u8>) -> Result<(), io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("dev.crs");

    write(&profile, &buffer)?;
    info!("CRS added to cache");

    Ok(())
}

pub fn delete_common_reference_string() -> Result<(), io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("dev.crs");

    remove_file(&profile)?;
    info!("CRS removed from cache");

    Ok(())
}

pub fn verify_common_reference_string(buff: &[u8]) -> bool {
    info!("Checking integrity of CRS");
    let mut hasher = Sha256::new();
    hasher.update(&buff);
    let hash = format!("{:x}", hasher.finalize());

    hash == CRS_17
}

pub fn get_lockfile_path() -> PathBuf {
    let mut dir = env::current_dir().expect("Failed to get current dir");
    let mut lockfile = dir.join("Cargo.lock");

    while !lockfile.exists() {
        if !dir.pop() {
            panic!("Failed to fetch Cargo.lock from upper dir!");
        }

        lockfile = dir.join("Cargo.lock");
    }

    lockfile
}

pub fn keys_for(crate_name: &str) -> Keys {
    use cargo_lock::{Lockfile, Package};

    let lockfile = get_lockfile_path();
    let lockfile = Lockfile::load(lockfile).expect("Cargo.lock not found!");

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

    info!("Getting keys for {} {}", crate_name, &version);

    Keys {
        crate_name: crate_name.to_string(),
        version,
    }
}
