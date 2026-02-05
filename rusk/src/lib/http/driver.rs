// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::ContractId;
use dusk_data_driver::reader::DriverReader;
use dusk_data_driver::{ConvertibleContract, Error, JsonValue};

/// A wrapper around [`DriverReader`] that implements [`ConvertibleContract`].
#[derive(Debug, Clone)]
pub struct DriverExecutor {
    reader: DriverReader,
}

impl DriverExecutor {
    /// Creates a new `DriverExecutor` from WASM bytecode.
    ///
    /// This loads and initializes the WASM module, calling `init()`
    /// automatically.
    pub fn from_bytecode(
        contract_id: &ContractId,
        bytecode: impl AsRef<[u8]>,
    ) -> anyhow::Result<Self> {
        let reader =
            DriverReader::with_contract_id(bytecode.as_ref(), *contract_id)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self { reader })
    }

    /// Returns the associated contract ID.
    pub fn contract_id(&self) -> ContractId {
        self.reader.contract_id()
    }

    /// Returns the contract driver's version string.
    pub fn get_version(&self) -> Result<String, Error> {
        self.reader.get_version()
    }
}

impl ConvertibleContract for DriverExecutor {
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        self.reader.encode_input_fn(fn_name, json)
    }

    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.reader.decode_input_fn(fn_name, rkyv)
    }

    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.reader.decode_output_fn(fn_name, rkyv)
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.reader.decode_event(event_name, rkyv)
    }

    fn get_schema(&self) -> String {
        self.reader
            .get_schema()
            .map(|v| v.to_string())
            .unwrap_or_default()
    }
}
