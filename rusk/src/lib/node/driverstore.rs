// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use dusk_core::abi::ContractId;

pub struct DriverStore {
    store_path: Option<PathBuf>,
}

impl DriverStore {
    pub fn new(path: Option<impl AsRef<Path>>) -> Self {
        Self {
            store_path: path.map(|p| p.as_ref().to_path_buf()),
        }
    }

    pub fn get_bytecode(
        &self,
        contract_id: &ContractId,
    ) -> io::Result<Option<Vec<u8>>> {
        Ok(match self.contract_path(contract_id) {
            Some(path) => {
                let mut f = File::open(&path)?;
                let metadata = fs::metadata(&path)?;
                let mut buffer = vec![0; metadata.len() as usize];
                f.read(&mut buffer)?;
                Some(buffer)
            }
            _ => None,
        })
    }

    pub fn store_bytecode(
        &mut self,
        contract_id: &ContractId,
        bytecode: &[u8],
    ) -> io::Result<()> {
        if let Some(path) = self.contract_path(contract_id) {
            let mut f = std::fs::File::create(&path)?;
            f.write_all(bytecode)?;
        }
        Ok(())
    }

    fn contract_path(&self, contract_id: &ContractId) -> Option<PathBuf> {
        self.store_path.as_ref().map(|path| {
            let contract_hex = hex::encode(contract_id);
            path.join(&contract_hex)
        })
    }

    pub fn length(&self) -> usize {
        1 // todo
    }
}
