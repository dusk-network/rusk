// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dirs::home_dir;
use std::{fs::{self, File, read, remove_dir, remove_file, write}};
use std::io::prelude::*;
use std::path::PathBuf;
use std::{ io};
use tracing::{info, warn};
use sha2::{Sha256, Digest};

static CRS_17: &str =
    "e1ebe5dedabf87d8fe1232e04d18a111530edc0f4beeeb0251d545a123d944fe";

#[derive(Debug, Clone)]
pub struct Keys {
    id: [u8;32]
}

impl Keys {
    pub fn get_prover(&self) -> Result<Vec<u8>, io::Error> {
        let mut dir = get_rusk_keys_dir()?;
        dir.push(format!("{}.pk", hex::encode(self.id)));

        match &dir.exists() {
            true => read(dir),
            false => Err(io::Error::new(io::ErrorKind::NotFound, "ProverKey not found"))
        }        
    }

    pub fn get_verifier(&self) -> Result<Vec<u8>, io::Error> {
        let mut dir = get_rusk_keys_dir()?;
        dir.push(format!("{}.vd", hex::encode(self.id)));

        match &dir.exists() {
            true => read(dir),
            false => Err(io::Error::new(io::ErrorKind::NotFound, "VerifierData not found"))
        }   
    }
}

pub fn get_rusk_profile_dir() -> Result<PathBuf, io::Error> {
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

pub fn clean_outdated_keys(ids: &Vec<[u8;32]>) -> Result<(), io::Error> {
    info!("Cleaning outdated keys (if any)");
    let ids_as_str: Vec<String> = ids.iter().map(|id| {
        hex::encode(id)
    }).collect(); 

    for entry in fs::read_dir(&get_rusk_keys_dir()?)? {
        let entry = entry?;
        let path = entry.path();
        if path.ends_with("vd") || path.ends_with("pk") {
            let id = path.file_stem().ok_or(io::Error::new(io::ErrorKind::Other, "Can't remove stem from file"))?
            .to_str().ok_or(io::Error::new(io::ErrorKind::Other, "Can't transform path to str"))?;

            if !ids_as_str.contains(&id.to_string()) {
                info!("Found file {:?} which is not included in the keys list obtained", path.clone());
                remove_file(path.clone())?;
                info!("{:?} was successfully removed outdated file", path);
            }
        }
    }

    info!("Cleaning outdated keys process completed successfully");
    Ok(())
}

pub fn keys_for(id: &[u8;32]) -> Result<Keys, io::Error> {
    let dir = get_rusk_keys_dir()?;
    let mut pk_dir = dir.clone();
    pk_dir.push(format!("{}.pk", hex::encode(id)));

    let mut vd_dir = dir.clone();
    vd_dir.push(format!("{}.vd", hex::encode(id)));


    if pk_dir.exists() || vd_dir.exists(){
        return Ok(Keys{id: *id})
    }   

    Err(io::Error::new(io::ErrorKind::NotFound, "keys not found"))
}
    

pub fn add_keys_for(id: &[u8;32], pk: Vec<u8>, vd: Vec<u8>) -> Result<(), io::Error> {
    let dir = get_rusk_keys_dir()?;

    let mut pk_file = dir.clone();
    pk_file.push(format!("{}.pk", hex::encode(id)));

    let mut vk_file = dir.clone();
    vk_file.push(format!("{}.vd", hex::encode(id)));

    File::create(pk_file)?.write_all(&pk)?;
    info!(
        "Entry added: {}.pk",
        hex::encode(id)
    );
    File::create(vk_file)?.write_all(&vd)?;
    info!(
        "Entry added: {}.vd",
        hex::encode(id)
    );

    Ok(())
}

pub fn clear_all_keys() -> Result<(), io::Error> {
    info!(
        "Clearing all the Keys folder contents"
    );
    
    fs::read_dir(&get_rusk_keys_dir()?)?
        .map(|res| res.map(|e| e.path()))
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap())
        .filter(|e| e.is_file())
        .filter(|p| match p.extension() {
            Some(os_str) => {
                match os_str.to_str() {
                    Some("pk" | "vd") => true,
                    _ => false,
                }
        },
            None => false
        })
        .map(|path| {
            info!("Removing {:?}", path.clone());
            remove_dir(path)
        }).collect::<Result<Vec<()>, io::Error>>()?;    

    Ok(())
}

