// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

use dusk_wasmtime::{Config, Engine, Instance, Module, Store};
use serde_json::Value;

use dusk_core::abi::ContractId;
use dusk_data_driver::{ConvertibleContract, Error, JsonValue};

fn config() -> Config {
    let mut config = Config::new();
    config.macos_use_mach_ports(false);
    config
}

pub struct DriverExecutor {
    store: Store<()>,
    instance: Option<Instance>,
}

impl DriverExecutor {
    pub fn new() -> Self {
        let config = config();
        let engine = Engine::new(&config)
            .expect("Wasmtime engine configuration should be valid");
        let store = Store::<()>::new(&engine, ());
        Self { store, instance: None }
    }

    pub fn load_bytecode(
        &mut self,
        contract_id: &ContractId,
        bytecode: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let module = Module::new(self.store.engine(), bytecode.as_ref())?;
        let instance = Instance::new(&mut self.store, &module, &[])?;
        self.instances.insert(*contract_id, instance);
        Ok(())
    }

    // pub fn exec() {
        // let gcd = instance.get_typed_func::<(i32, i32), i32>(&mut store,
        // "gcd")?; gcd.call(&mut store, (6, 27))?;
    // }
}

impl ConvertibleContract for DriverExecutor {
    fn encode_input_fn(&self, fn_name: &str, json: &str) -> Result<Vec<u8>, Error> {
        // gcd.call(&mut store, (6, 27))?;

        // fn_name_ptr: *mut u8,  // alloc
        // fn_name_size: usize,
        // json_ptr: *mut u8,     // alloc
        // json_size: usize,
        // out_ptr: *mut u8,      // alloc
        // out_buf_size: usize,

        let instance = self.instance.expect("instance should exist in executor");

        // allocate memory for fn_name_size into fn_name_buf
        let alloc = instance.get_typed_func::<(usize), *mut u8>(&mut store, "alloc")?;
        let fn_name_buf = alloc.call(&mut store, fn_name_size)?;

        // allocate memory for json_size into json_buf
        let json_buf = alloc.call(&mut store, json_size)?;

        // allocate memory for out_buf_size into out_buf
        const OUT_BUF_SIZE: usize = 8192;
        let out_buf = alloc.call(&mut store, OUT_BUF_SIZE)?;

        // copy fn_name to fn_name_buf

        // copy json_ptr to json_buf

        // call encode_input_fn with the following arguments:
        // fn_name_buf
        // fn_name.len()
        // json_buf
        // json.len()
        // out_buf
        // 8192 - good question, what should it be?

        // dealloc of fn_name_buf
        // dealloc of json_buf

        // copy the output buffer to vector

        // dealloc of out_buf

        let f = instance.get_typed_func::<(i32, i32), i32>(&mut store, "encode_input_fn")?;
        f.call(&mut store, )?;
        Ok(Vec::new())
    }

    fn decode_input_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        Ok(JsonValue::Null)
    }

    fn decode_output_fn(&self, fn_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        Ok(JsonValue::Null)
    }

    fn decode_event(&self, event_name: &str, rkyv: &[u8]) -> Result<JsonValue, Error> {
        Ok(JsonValue::Null)
    }

    fn get_schema(&self) -> String {
        "".to_string()
    }
}
