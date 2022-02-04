// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::{fmt, fs};

use blake3::Hash;
use dusk_wallet_core::Store;

use crate::lib::crypto::{decrypt, encrypt};
use crate::lib::SEED_SIZE;
use crate::Error;

/// Binary prefix for Dusk wallet files
const MAGIC: &[u8] = &[21, 12, 29];

/// Specifies the encoding used to save files
const VERSION: &[u8] = &[1, 0];

/// Stores all the user's settings and keystore in the file system
#[derive(Clone)]
pub struct LocalStore {
    path: PathBuf,
    seed: [u8; SEED_SIZE],
}

impl Store for LocalStore {
    type Error = Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; SEED_SIZE], Self::Error> {
        Ok(self.seed)
    }
}

impl LocalStore {
    /// Creates a new store
    pub fn new(
        path: PathBuf,
        seed: [u8; SEED_SIZE],
    ) -> Result<LocalStore, Error> {
        // create the local store
        let store = LocalStore { path, seed };

        Ok(store)
    }

    /// Loads wallet file from file
    pub fn from_file(path: PathBuf, pwd: Hash) -> Result<LocalStore, Error> {
        // basic sanity check
        if !path.is_file() {
            return Err(Error::WalletFileNotExists);
        }

        // attempt to load and decode wallet
        let mut bytes = fs::read(&path)?;

        // check for magic number
        for i in 0..3 {
            if bytes[i] != MAGIC[i] {
                return Self::from_legacy_file(path, pwd);
            }
        }
        bytes.drain(0..3);

        // check for version information
        let version = format!("{}.{}", bytes[0], bytes[1]);
        bytes.drain(0..2);

        // decrypt and interpret file contents
        let mut seed = [0u8; SEED_SIZE];
        match version.as_str() {
            "1.0" => {
                bytes = decrypt(&bytes, pwd)?;
                if bytes.len() != SEED_SIZE {
                    return Err(Error::WalletFileCorrupted);
                }
                seed.copy_from_slice(&bytes);
            }
            _ => {
                return Err(Error::UnknownFileVersion);
            }
        };

        // create and return
        Ok(LocalStore { path, seed })
    }

    /// Attempts to load a legacy wallet file (no version number)
    fn from_legacy_file(path: PathBuf, pwd: Hash) -> Result<LocalStore, Error> {
        // attempt to load and decode wallet
        let mut bytes = fs::read(&path)?;

        // check for old version information and strip it if present
        if bytes[1] == 0 && bytes[2] == 0 {
            bytes.drain(0..3);
        }

        bytes = decrypt(&bytes, pwd)?;
        if bytes.len() != SEED_SIZE {
            return Err(Error::WalletFileCorrupted);
        }

        // get our seed
        let mut seed = [0u8; SEED_SIZE];
        seed.copy_from_slice(&bytes);

        // return the store
        Ok(LocalStore { path, seed })
    }

    /// Saves wallet to a file
    pub fn save(&self, pwd: Hash) -> Result<(), Error> {
        // encrypt seed
        let enc_seed = encrypt(&self.seed, pwd)?;

        // create file
        let mut content = enc_seed;

        // prepend magic number and encoding version information
        let mut prefix = [0u8; MAGIC.len() + VERSION.len()];
        prefix[..MAGIC.len()].copy_from_slice(MAGIC);
        prefix[MAGIC.len()..].copy_from_slice(VERSION);
        content.splice(0..0, prefix.iter().cloned());

        // write file
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Returns the filename of this store
    pub fn name(&self) -> Option<String> {
        // extract the name
        let p = &self.path;
        let name = p.file_stem()?.to_str()?;

        Some(String::from(name))
    }
}

impl fmt::Debug for LocalStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "-------------------\n\
            LocalStore: {}\n\
            Seed: {:?}",
            self.path.as_os_str().to_str().unwrap(),
            self.seed
        )
    }
}
