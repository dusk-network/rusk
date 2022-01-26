// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;
use std::{fmt, fs};

use dusk_bls12_381_sign::SecretKey;
use dusk_pki::SecretSpendKey;
use dusk_wallet_core::{derive_sk, derive_ssk, Store};

use crate::lib::crypto::{decrypt_seed, encrypt_seed};
use crate::lib::errors::CliError;

/// Stores all the user's settings and keystore in the file system
pub struct LocalStore {
    path: PathBuf,
    seed: [u8; 64],
}

impl Store for LocalStore {
    type Error = CliError;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }

    /// Retrieves a derived secret spend key from the store.
    fn retrieve_ssk(&self, index: u64) -> Result<SecretSpendKey, Self::Error> {
        Ok(derive_ssk(&self.seed, index))
    }

    /// Retrieves a derived secret key from the store.
    fn retrieve_sk(&self, index: u64) -> Result<SecretKey, CliError> {
        Ok(derive_sk(&self.seed, index))
    }
}

impl LocalStore {
    /// Creates a new store
    pub fn new(path: PathBuf, seed: [u8; 64]) -> Result<LocalStore, CliError> {
        // create the local store
        let store = LocalStore { path, seed };

        Ok(store)
    }

    /// Loads wallet file from file
    pub fn from_file(
        path: PathBuf,
        pwd: String,
    ) -> Result<LocalStore, CliError> {
        // basic sanity check
        if !path.is_file() {
            return Err(CliError::FileNotExists);
        }

        // attempt to load and decode wallet
        let seed_bytes = decrypt_seed(fs::read(&path)?, fs::read(&path)?, pwd)?;

        // wallet_seed
        let mut seed = [0u8; 64];
        seed.copy_from_slice(&seed_bytes);

        // create and return
        Ok(LocalStore { path, seed })
    }

    /// Saves wallet to a file
    pub fn save(&self, pwd: String) -> Result<(), CliError> {
        // encrypt seed
        let (data, iv) = encrypt_seed(&self.seed, pwd)?;

        // write file
        fs::write(&self.path, &data)?;
        fs::write(&self.path, &iv)?;
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
