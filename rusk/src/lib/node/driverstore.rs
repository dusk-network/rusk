// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use std::path::{Path, PathBuf};
use std::{fs, io};

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::Signature;

const SIGNATURE_FILE_EXTENSION: &str = "sign";

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
        self.contract_path(contract_id).map(fs::read).transpose()
    }

    pub fn get_signature(
        &self,
        contract_id: &ContractId,
    ) -> io::Result<Option<Vec<u8>>> {
        self.contract_path(contract_id)
            .map(|path| fs::read(path.with_extension(SIGNATURE_FILE_EXTENSION)))
            .transpose()
    }

    pub fn store_bytecode_and_signature(
        &mut self,
        contract_id: &ContractId,
        bytecode: &[u8],
        signature_bytes: [u8; Signature::SIZE],
    ) -> io::Result<()> {
        if let Some(path) = self.contract_path(contract_id) {
            fs::write(&path, bytecode)?;
            fs::write(
                path.with_extension(SIGNATURE_FILE_EXTENSION),
                signature_bytes,
            )?;
        }
        Ok(())
    }

    fn contract_path(&self, contract_id: &ContractId) -> Option<PathBuf> {
        self.store_path.as_ref().map(|path| {
            let contract_hex = hex::encode(contract_id);
            path.join(&contract_hex)
        })
    }
}
