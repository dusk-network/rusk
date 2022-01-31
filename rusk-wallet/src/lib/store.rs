// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::{fmt, fs};

use blake3::Hash;
use dusk_wallet_core::Store;

use crate::lib::crypto::EncryptedSeed;
use crate::Error;

/// Stores all the user's settings and keystore in the file system
pub struct LocalStore {
    path: PathBuf,
    seed: [u8; 64],
}

impl Store for LocalStore {
    type Error = Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }
}

impl LocalStore {
    /// Creates a new store
    pub fn new(path: PathBuf, seed: [u8; 64]) -> Result<LocalStore, Error> {
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
        let bytes = fs::read(&path)?;
        if bytes.len() != EncryptedSeed::SIZE {
            return Err(Error::WalletFileCorrupted);
        }
        let mut seed_bytes = [0u8; EncryptedSeed::SIZE];
        seed_bytes.copy_from_slice(&bytes);

        let seed = EncryptedSeed::from_bytes(&seed_bytes);
        let seed = seed.decrypt(pwd)?;

        // create and return
        Ok(LocalStore { path, seed })
    }

    /// Saves wallet to a file
    pub fn save(&self, pwd: Hash) -> Result<(), Error> {
        // encrypt seed
        let mut seed = EncryptedSeed::from_seed(self.seed);
        seed.encrypt(pwd)?;

        // write file
        fs::write(&self.path, &seed.to_bytes())?;
        Ok(())
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
