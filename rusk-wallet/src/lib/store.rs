// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::Read;
use std::{fmt, fs, io};
use std::path::PathBuf;
use bytes::{BytesMut, BufMut, Bytes, Buf};

use dusk_pki::SecretSpendKey;
use dusk_wallet_core::{Store, generate_ssk};

/// Default directory name to store settings and keystore
const DATA_DIR: &str = ".dusk";

/// Error types returned by this crate
#[derive(Debug)]
pub enum StoreError {
    IO(io::Error),
    CorruptedFile,
    KeyNotFound,
    KeyAlreadyExists,
    InvalidPhrase,
}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

/// Stores all the user's settings and keystore in the file system
pub struct LocalStore {
    name: String,
    path: PathBuf,
    seed: [u8; 64],
    keys: Vec<u64>
}

impl Store for LocalStore {

    type Error = StoreError;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }

    /// Retrieves a stored key for this wallet.
    fn retrieve_key(&self, index: u64) -> Result<SecretSpendKey, Self::Error> {
        match self.has_key(index) {
            true => Ok(generate_ssk(&self.seed, index)),
            false => Err(StoreError::KeyNotFound),
        }
    }

}

impl LocalStore {

    /// Creates a new store
    pub fn new(name: String, seed: [u8;64]) -> Result<LocalStore, StoreError> {

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
            keys: vec![],
        };

        // save to disk and return
        store.save()?;
        Ok(store)

    }


    /// Loads wallet file from file
    pub fn from_file(path: PathBuf, _pwd: String) -> Result<LocalStore, StoreError> {

        // basic sanity check
        if !path.is_file() {
            return Err(StoreError::CorruptedFile);
        }

        // attempt to load and decode wallet
        let data = Bytes::from(fs::read(&path).unwrap());
        let mut reader = data.reader();
    
        // wallet_seed
        let mut wallet_seed= [0u8; 64];
        reader.read(&mut wallet_seed)?;
    
        // key_count
        let mut key_count = [0u8; 8];
        reader.read(&mut key_count)?;
        let key_count = usize::from_le_bytes(key_count);
    
        // keys
        let mut keys = vec![];
        for _ in 0..key_count {

            // index
            let mut index = [0u8; 8];
            reader.read(&mut index)?;
            let index = u64::from_le_bytes(index);

            keys.push(index)
        }
    
        // extract the name
        let name = path.file_name().ok_or(StoreError::CorruptedFile)?.to_str().ok_or(StoreError::CorruptedFile)?;

        // create and return
        Ok(LocalStore{
            name: String::from(name),
            path: path,
            seed: wallet_seed,
            keys: keys,
        })

    }

    /// Saves wallet to a file
    pub fn save(&self) -> Result<(), StoreError> {

        let mut buf = BytesMut::new();
    
        // wallet seed
        buf.put_slice(&self.seed);

        // key count
        let len = self.keys.len();
        buf.put_slice(&len.to_le_bytes());

        // keys
        for key in self.keys.iter() {
            buf.put_slice(&key.to_le_bytes());
        }
    
        // write file
        fs::write(&self.path, &buf[..]).unwrap();
        Ok(())

    }

    /// Add a new secret key to the wallet
    /// Note: This method does not persist the changes to disk.
    pub fn add_key(&mut self, index: u64) -> Result<SecretSpendKey, StoreError> {
        match self.has_key(index) {
            true => Err(StoreError::KeyAlreadyExists),
            false => {
                let ssk = generate_ssk(&self.seed, index);
                self.keys.push(index);
                Ok(ssk)
            },
        }
    }

    /// Remove an existing secret key from the wallet.
    /// Note: This method does not persist the changes to disk.
    pub fn remove_key(&mut self, index: u64) {
        self.keys.retain(|key| *key != index)
    }

    /// Check if a key exists in this store.
    pub fn has_key(&self, index: u64) -> bool {
        self.keys.contains(&index)
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
            Seed: {:?}\n\
            Keys: {}\n",
            self.path.as_os_str().to_str().unwrap(), 
            self.name, self.seed, self.keys.len())
    }
}