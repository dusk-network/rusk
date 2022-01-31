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
        let bytes = fs::read(&path)?;
        let bytes = decrypt(&bytes, pwd)?;

        if bytes.len() != SEED_SIZE {
            return Err(Error::WalletFileCorrupted);
        }

        let mut seed = [0u8; SEED_SIZE];
        seed.copy_from_slice(&bytes);

        // create and return
        Ok(LocalStore { path, seed })
    }

    /// Saves wallet to a file
    pub fn save(&self, pwd: Hash) -> Result<(), Error> {
        // encrypt seed
        let enc_seed = encrypt(&self.seed, pwd)?;

        // write file
        fs::write(&self.path, enc_seed)?;
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
