// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};

use dusk_core::abi::ContractId;

use crate::Result;

#[derive(Serialize, Deserialize, Clone)]
pub struct DriverStoreConfig2 {
    pub driver_store_path: Option<PathBuf>,
    pub driver_store_limit: u64,
}

impl Default for DriverStoreConfig2 {
    fn default() -> Self {
        Self {
            driver_store_path: None,
            driver_store_limit: default_driver_store_limit(),
        }
    }
}

const fn default_driver_store_limit() -> u64 {
    1024
}

pub struct DriverStore {
    store_path: Option<PathBuf>,
    store_limit: u64,
}

impl DriverStore {
    pub fn new(path: Option<impl AsRef<Path>>, limit: u64) -> Self {
        Self {
            store_path: path,
            store_limit: limit,
        }
    }

    pub fn get_bytecode(&self, contract_id: &ContractId) -> Result<Vec<u8>> {
        let path = self.contract_path(contract_id).expect("contract path should exist");
        let mut f = File::open(&path)?;
        let metadata = fs::metadata(&path)?;
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer)?;
        Ok(buffer)
    }

    pub fn store_bytecode(&mut self, contract_id: &ContractId, bytecode: &[u8]) -> Result<()> {
        self.contract_path(contract_id).map(|path| {
            let mut f = std::fs::File::create(path)?;
            f.write_all(bytecode)?;
        });
        Ok(())
    }

    fn contract_path(&self, contract_id: &ContractId) -> Option<PathBuf> {
        self.store_path.map(|path|{
            let contract_hex = hex::encode(contract_id);
            path.join(&contract_hex)
        })
    }

    pub fn store_limit(&self) -> u64 {
        self.store_limit
    }
}
