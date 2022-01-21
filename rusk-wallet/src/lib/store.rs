// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::Read;
use std::{fmt, fs, io};
use std::path::PathBuf;
use bytes::{BytesMut, BufMut, Bytes, Buf};

use dusk_pki::{SecretSpendKey, SecretKey};
use dusk_wallet_core::{Store, derive_sk, derive_ssk};

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
    ssk_keys: Vec<u64>,
    sk_keys: Vec<u64>
}

impl Store for LocalStore {

    type Error = StoreError;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok(self.seed)
    }

    /// Retrieves a derived secret spend key from the store.
    fn retrieve_ssk(&self, index: u64) -> Result<SecretSpendKey, Self::Error> {
        match self.has_ssk(index) {
            true => Ok(derive_ssk(&self.seed, index)),
            false => Err(StoreError::KeyNotFound),
        }
    }

    /// Retrieves a derived secret key from the store.
    fn retrieve_sk(&self, index: u64) -> Result<SecretKey, Self::Error> {
        match self.has_sk(index) {
            true => Ok(derive_sk(&self.seed, index)),
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
            ssk_keys: vec![],
            sk_keys: vec![],
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
    
        // ssk key count
        let mut key_count = [0u8; 8];
        reader.read(&mut key_count)?;
        let key_count = usize::from_le_bytes(key_count);
    
        // ssk keys
        let mut ssk_keys = vec![];
        for _ in 0..key_count {
            let mut index = [0u8; 8];
            reader.read(&mut index)?;
            let index = u64::from_le_bytes(index);
            ssk_keys.push(index)
        }
    
        // sk key count
        let mut key_count = [0u8; 8];
        reader.read(&mut key_count)?;
        let key_count = usize::from_le_bytes(key_count);
    
        // sk keys
        let mut sk_keys = vec![];
        for _ in 0..key_count {
            let mut index = [0u8; 8];
            reader.read(&mut index)?;
            let index = u64::from_le_bytes(index);
            sk_keys.push(index)
        }

        // extract the name
        let name = path.file_name().ok_or(StoreError::CorruptedFile)?.to_str().ok_or(StoreError::CorruptedFile)?;

        // create and return
        Ok(LocalStore{
            name: String::from(name),
            path: path,
            seed: wallet_seed,
            sk_keys: sk_keys,
            ssk_keys: ssk_keys,
        })

    }

    /// Saves wallet to a file
    pub fn save(&self) -> Result<(), StoreError> {

        let mut buf = BytesMut::new();
    
        // wallet seed
        buf.put_slice(&self.seed);

        // ssk key count
        let len = self.ssk_keys.len();
        buf.put_slice(&len.to_le_bytes());

        // ssk keys
        for key in self.ssk_keys.iter() {
            buf.put_slice(&key.to_le_bytes());
        }

        // sk key count
        let len = self.sk_keys.len();
        buf.put_slice(&len.to_le_bytes());

        // sk keys
        for key in self.sk_keys.iter() {
            buf.put_slice(&key.to_le_bytes());
        }
    
        // write file
        fs::write(&self.path, &buf[..]).unwrap();
        Ok(())

    }

    /// Add a new secret spend key to the wallet
    /// Note: This method does not persist the changes to disk.
    pub fn add_ssk(&mut self, index: u64) -> Result<SecretSpendKey, StoreError> {
        match self.has_ssk(index) {
            true => Err(StoreError::KeyAlreadyExists),
            false => {
                let ssk = derive_ssk(&self.seed, index);
                self.ssk_keys.push(index);
                Ok(ssk)
            },
        }
    }

    /// Remove an existing secret spend key from the wallet.
    /// Note: This method does not persist the changes to disk.
    pub fn remove_ssk(&mut self, index: u64) {
        self.ssk_keys.retain(|key| *key != index)
    }

    /// Check if a secret spend key exists in this store.
    pub fn has_ssk(&self, index: u64) -> bool {
        self.ssk_keys.contains(&index)
    }

    /// Add a new secret key to the wallet
    /// Note: This method does not persist the changes to disk.
    pub fn add_sk(&mut self, index: u64) -> Result<SecretKey, StoreError> {
        match self.has_sk(index) {
            true => Err(StoreError::KeyAlreadyExists),
            false => {
                let sk = derive_sk(&self.seed, index);
                self.sk_keys.push(index);
                Ok(sk)
            },
        }
    }

    /// Remove an existing secret key from the wallet.
    /// Note: This method does not persist the changes to disk.
    pub fn remove_sk(&mut self, index: u64) {
        self.sk_keys.retain(|key| *key != index)
    }

    /// Check if a secret key exists in this store.
    pub fn has_sk(&self, index: u64) -> bool {
        self.sk_keys.contains(&index)
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
            SSK Keys: {}\n\
            SK Keys: {}\n",
            self.path.as_os_str().to_str().unwrap(), 
            self.name, self.seed, self.ssk_keys.len(),
            self.sk_keys.len())
    }
}