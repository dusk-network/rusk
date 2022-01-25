// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::Read;
use std::{fmt, fs};
use std::path::PathBuf;
use bytes::{BytesMut, BufMut, Bytes, Buf};

use dusk_pki::{SecretSpendKey, SecretKey};
use dusk_wallet_core::{Store, derive_sk, derive_ssk};

use crate::lib::errors::CliError;

/// Default directory name to store settings and keystore
const DATA_DIR: &str = ".dusk";

/// Stores all the user's settings and keystore in the file system
pub struct LocalStore {
    name: String,
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
    fn retrieve_sk(&self, index: u64) -> Result<SecretKey, Self::Error> {
        Ok(derive_sk(&self.seed, index))
    }

}

impl LocalStore {

    /// Creates a new store
    pub fn new(name: String, seed: [u8;64]) -> Result<LocalStore, CliError> {

        // construct path to file
        let home = dirs::home_dir().unwrap();
        let mut path = PathBuf::new();
        path.push(home.as_os_str());
        path.push(DATA_DIR);
        path.push(&name);
        path = path.with_extension("dat");

        // get the seed
        let mut seed_bytes= [0u8; 64];
        seed_bytes.copy_from_slice(&seed);

        // create the local store
        let store = LocalStore{
            name: name,
            path: path,
            seed: seed_bytes,
        };

        // save to disk and return
        store.save()?;
        Ok(store)

    }


    /// Loads wallet file from file
    pub fn from_file(path: PathBuf, _pwd: String) -> Result<LocalStore, CliError> {

        // basic sanity check
        if !path.is_file() {
            return Err(CliError::CorruptedFile);
        }

        // attempt to load and decode wallet
        let data = Bytes::from(fs::read(&path).unwrap());
        let mut reader = data.reader();

        // wallet_seed
        let mut wallet_seed = [0u8; 64];
        reader.read(&mut wallet_seed)?;

        // extract the name
        let name = path.file_name()
            .ok_or(CliError::CorruptedFile)?.to_str()
            .ok_or(CliError::CorruptedFile)?;

        // create and return
        Ok(LocalStore{
            name: String::from(name),
            path: path,
            seed: wallet_seed,
        })

    }

    /// Saves wallet to a file
    pub fn save(&self) -> Result<(), CliError> {

        let mut buf = BytesMut::new();

        // wallet seed (todo: encrypt)
        buf.put_slice(&self.seed);

        // write file
        fs::write(&self.path, &buf[..]).unwrap();
        Ok(())

    }

    /// The name of the wallet
    pub fn name(&self) -> String {
        self.name.clone()
    }

}

impl fmt::Debug for LocalStore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "-------------------\n\
            LocalStore: {}\n\
            Name: {}\n\
            Seed: {:?}",
            self.path.as_os_str().to_str().unwrap(), 
            self.name, self.seed)
    }
}