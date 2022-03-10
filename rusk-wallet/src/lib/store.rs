// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::{Path, PathBuf};
use std::{fmt, fs};

use blake3::Hash;
use dusk_wallet_core::Store;

use crate::lib::crypto::{decrypt, encrypt};
use crate::lib::SEED_SIZE;
use crate::StoreError;

/// Default data directory name
pub(crate) const DATA_DIR: &str = ".dusk";
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
    type Error = StoreError;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; SEED_SIZE], Self::Error> {
        Ok(self.seed)
    }
}

impl LocalStore {
    /// Data directory defaults to user's home dir
    pub fn default_data_dir() -> PathBuf {
        let home = dirs::home_dir().expect("OS not supported");
        Path::new(home.as_os_str()).join(DATA_DIR)
    }

    /// Wallet name defaults to user's username
    pub fn default_wallet_name() -> String {
        // get default user as default wallet name (remove whitespace)
        let mut user: String = whoami::username();
        user.retain(|c| !c.is_whitespace());
        user.push_str(".dat");
        user
    }

    /// Default fully-quallified wallet file
    pub fn default_wallet() -> PathBuf {
        let mut pb = PathBuf::new();
        pb.push(Self::default_data_dir());
        pb.push(Self::default_wallet_name());
        pb.set_extension("dat");
        pb
    }

    /// Creates directories
    pub fn create_dir(dir: &PathBuf) -> Result<(), StoreError> {
        fs::create_dir_all(dir)?;
        Ok(())
    }

    /// Checks if a wallet with this name already exists
    pub fn wallet_exists(name: &str) -> bool {
        let mut pb = PathBuf::new();
        pb.push(Self::default_data_dir());
        pb.push(name);
        pb.set_extension("dat");
        pb.is_file()
    }

    /// Creates a new store
    pub fn new(
        path: PathBuf,
        seed: [u8; SEED_SIZE],
    ) -> Result<LocalStore, StoreError> {
        // create the local store
        let store = LocalStore { path, seed };

        Ok(store)
    }

    /// Scan data directory and return a list of wallet names
    pub fn find_wallets(dir: &PathBuf) -> Result<Vec<String>, StoreError> {
        let dir = fs::read_dir(dir)?;

        let wallets = dir
            .filter_map(|el| el.ok().map(|d| d.path()))
            .filter(|path| path.is_file())
            .filter(|path| match path.extension() {
                Some(ext) => ext == "dat",
                None => false,
            })
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(String::from)
            })
            .collect();

        Ok(wallets)
    }

    /// Loads wallet file from file
    pub fn from_file(
        path: PathBuf,
        pwd: Hash,
    ) -> Result<LocalStore, StoreError> {
        // basic sanity check
        let mut path = path;
        if path.extension().is_none() {
            path.set_extension("dat");
        }

        // make sure file exists
        if !path.is_file() {
            return Err(StoreError::WalletFileNotExists);
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
                    return Err(StoreError::WalletFileCorrupted);
                }
                seed.copy_from_slice(&bytes);
            }
            _ => {
                return Err(StoreError::UnknownFileVersion);
            }
        };

        // create and return
        Ok(LocalStore { path, seed })
    }

    /// Attempts to load a legacy wallet file (no version number)
    fn from_legacy_file(
        path: PathBuf,
        pwd: Hash,
    ) -> Result<LocalStore, StoreError> {
        // attempt to load and decode wallet
        let mut bytes = fs::read(&path)?;

        // check for old version information and strip it if present
        if bytes[1] == 0 && bytes[2] == 0 {
            bytes.drain(0..3);
        }

        bytes = decrypt(&bytes, pwd)?;
        if bytes.len() != SEED_SIZE {
            return Err(StoreError::WalletFileCorrupted);
        }

        // get our seed
        let mut seed = [0u8; SEED_SIZE];
        seed.copy_from_slice(&bytes);

        // return the store
        Ok(LocalStore { path, seed })
    }

    /// Saves wallet to a file
    pub fn save(&self, pwd: Hash) -> Result<(), StoreError> {
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

    /// Returns current directory for this store
    pub fn dir(&self) -> Option<PathBuf> {
        let mut p = self.path.clone();
        if p.pop() {
            Some(p)
        } else {
            None
        }
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

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_localstore() -> Result<(), StoreError> {
        // create a wallet
        let path = PathBuf::from("/tmp/test_wallet.dat");
        let seed = [123u8; 64];
        let st = LocalStore::new(path.clone(), seed)?;

        // store it on disk
        let pwd = blake3::hash("mypassword".as_bytes());
        st.save(pwd)?;

        // load it back
        let loaded = LocalStore::from_file(path, pwd)?;

        // check name
        match loaded.name() {
            Some(name) => assert_eq!(name, "test_wallet"),
            None => panic!("no wallet name"),
        }

        // check seed
        let lseed = loaded.get_seed()?;
        assert_eq!(lseed, seed);

        Ok(())
    }
}
