// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use std::{fs::{self, read, remove_file, write, File, OpenOptions}};
use std::io::prelude::*;
use std::path::PathBuf;
use std::{ io};
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use bincode::{serialize, deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig(HashMap<[u8;32], Keys>);

impl KeysConfig {
    fn read_keys_config() -> Result<Self, io::Error> {
        let file = get_keys_config()?;
        let buff = match read(file) {
            Ok(buff) => buff,
            _ => {
                warn!("Error reading KeysConfig. NotFound.");
                let new_conf = KeysConfig(HashMap::new());
                info!("Created new empty KeysConfig");
                return Ok(new_conf)
            }
        };

        let config: KeysConfig = match deserialize(&buff) {
            Ok(conf) => conf,
            _ => {
                warn!("Error reading KeysConfig. NotFound.");
                let new_conf = KeysConfig(HashMap::new());
                info!("Created new empty KeysConfig");
                new_conf
            }
        };
        Ok(config)
    }

    fn write_keys_config(&self) -> Result<(), io::Error> {
        let mut file = File::create(get_keys_config()?)?;
        file.write_all(&serialize(&self).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{:?}", e)))?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keys {
    id: [u8;32],
    crate_name: String,
    label: Option<String>,
    // dir has the format `.rusk/keys/crate_namelabel(if exists)`
    dir: PathBuf,
}

impl Keys {
    pub fn dir(&self) -> Option<PathBuf> {
        if let Ok(mut dir) = get_rusk_keys_dir() {
            info!("Found the rusk keys dir");

            dir.push(&self.crate_name);
            dir.push(&self.label.clone().unwrap_or("".to_string()));
            Some(dir)
        } else {
            info!("Couldn't find the rusk keys dir");

            None
        }
    }

    pub fn are_outdated(&self) -> bool {
        let outdated = self.dir().map_or(true, |dir| !dir.exists());
        info!("keys outdated: {}", outdated);
        outdated
    }

    /* UNUSED
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
    }*/

    pub fn get_prover(&self) -> Option<Vec<u8>> {
        let dir = self.dir();

        dir.filter(|dir| dir.exists()).and_then(|mut dir| {
            dir.push(format!("{}.pk", hex::encode(self.id)));

            read(dir).ok()
        })
    }

    pub fn get_verifier(&self) -> Option<Vec<u8>> {
        let dir = self.dir();

        dir.filter(|dir| dir.exists()).and_then(|mut dir| {
            dir.push(format!("{}.vd", hex::encode(self.id)));

            read(dir).ok()
        })
    }

    /* UNUSED
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
    }*/

    pub fn update(
        &self,
        keys: (Vec<u8>, Vec<u8>),
    ) -> Result<(), io::Error> {
        let mut dir = get_rusk_keys_dir()?;

        dir.push(&self.crate_name);
        dir.push(&self.label.clone().unwrap_or("".to_string()));
        fs::create_dir_all(dir.clone())?;

        let mut pk_file = dir.clone();
        pk_file.push(format!("{}.pk", hex::encode(self.id)));

        let mut vk_file = dir;
        vk_file.push(format!("{}.vd", hex::encode(self.id)));

        File::create(pk_file)?.write_all(&keys.0)?;
        File::create(vk_file)?.write_all(&keys.1)?;

        info!(
            "Cache updated for VD and PK of {} with Id: {} ",
            &self.crate_name, hex::encode(self.id)
        );

        Ok(())
    }
}

fn get_rusk_profile_dir() -> Result<PathBuf, io::Error> {
    if let Some(mut dir) = home_dir() {
        dir.push(".rusk");
        fs::create_dir_all(dir.clone())?;
        Ok(dir)
    } else {
        warn!("rusk-profile dir not found and impossible to create");
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "User Profile Dir not found",
        ))
    }
}

fn get_rusk_keys_dir() -> Result<PathBuf, io::Error> {
    let mut profile = get_rusk_profile_dir()?;
    profile.push("keys");
    fs::create_dir_all(profile.clone())?;
    Ok(profile)
}

fn get_keys_config() -> Result<PathBuf, io::Error> {
    let mut profile = get_rusk_keys_dir()?;
    profile.push(".config.bin");
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

/* UNUSED
fn get_lockfile_path() -> PathBuf {
    let mut dir = env::current_dir().expect("Failed to get current dir");
    let mut lockfile = dir.join("Cargo.lock");

    while !lockfile.exists() {
        if !dir.pop() {
            panic!("Failed to fetch Cargo.lock from upper dir!");
        }

        lockfile = dir.join("Cargo.lock");
    }

    lockfile
}*/

pub fn keys_for(id: &[u8;32]) -> Result<Option<Keys>, io::Error> {
    let conf = KeysConfig::read_keys_config()?;

    Ok(conf.0.get(id).cloned())
}

pub fn add_keys_for(id: &[u8;32], crate_name: &str, label: Option<String>, pk: Vec<u8>, vd: Vec<u8>) -> Result<(), io::Error> {
    let mut conf = KeysConfig::read_keys_config()?;

    let mut key = Keys {
        id: *id,
        crate_name: crate_name.into(),
        label,
        dir: PathBuf::from("/whatever")
    };

    key.dir = match key.dir() {
        Some(dir) => dir,
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "Rusk dir not found"))
    };
    fs::create_dir_all(key.dir.clone())?;
    info!("Created dir {:?} to store new keys", key.dir);

    let mut pk_file = key.dir.clone();
    pk_file.push(format!("{}.pk", hex::encode(id)));

    let mut vk_file = key.dir.clone();
    vk_file.push(format!("{}.vd", hex::encode(id)));

    File::create(pk_file)?.write_all(&pk)?;
    File::create(vk_file)?.write_all(&vd)?;

    info!(
        "Entry added for {}-circuits with ID: {} ",
        crate_name, hex::encode(id)
    );

    conf.0.insert(*id, key);
    conf.write_keys_config()?;
    Ok(())
}