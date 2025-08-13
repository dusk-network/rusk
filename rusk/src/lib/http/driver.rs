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

    pub fn allocate(&mut self, sz: usize) -> Result<*mut u8, Error> {
        let instance = self.instance.expect("instance should exist in executor");
        let alloc = instance.get_typed_func::<(usize), *mut u8>(&mut self.store, "alloc")?;
        let mem = alloc.call(&mut self.store, sz)?;
        Ok(mem)
    }

    pub fn allocate_and_copy(&mut self, bytes: &[u8], sz: usize) -> Result<*mut u8, Error> {
        let mem = self.allocate(sz)?;
        let dst_slice = unsafe { std::slice::from_raw_parts_mut(mem, sz) };
        dst_slice.copy_from_slice(&bytes[..sz]);
        Ok(mem)
    }

    pub fn deallocate(&mut self, ptr: *mut u8, sz: usize) -> Result<(), Error> {
        let instance = self.instance.expect("instance should exist in executor");
        let dealloc = instance.get_typed_func::<(*mut u8, usize), ()>(&mut self.store, "dealloc")?;
        dealloc.call(&mut self.store, (ptr, sz))?;
        Ok(())
    }
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

        let fn_name_ptr = self.allocate_and_copy(fn_name.as_bytes(), fn_name.len())?;
        let json_ptr = self.allocate_and_copy(json.as_bytes(), json.len())?;
        const OUT_BUF_SIZE: usize = 8192; // todo: 8192 - good question, what should it be?
        let out_ptr = self.allocate(OUT_BUF_SIZE)?;

        let f = instance.get_typed_func::<(*mut u8, usize, *mut u8, usize, *mut u8, usize), i32>(&mut self.store, "encode_input_fn")?;
        let error_code = f.call(&mut self.store, (fn_name_ptr, fn_name.len(), json_ptr, json.len(), out_ptr, OUT_BUF_SIZE))?;

        self.deallocate(fn_name_ptr, fn_name.len());
        self.deallocate(json_ptr, json.len());

        let out_slice = unsafe { std::slice::from_raw_parts(out_ptr, OUT_BUF_SIZE) };
        let mut out_vector = Vec::new();
        out_vector.extend_from_slice(&out_slice);
        self.deallocate(out_ptr, OUT_BUF_SIZE);
        Ok(out_vector)
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
