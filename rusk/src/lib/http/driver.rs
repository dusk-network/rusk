// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::ptr;

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


fn read_u32_be_and_bytes(p: *const u8) -> (u32, Vec<u8>) {
    // SAFETY: We assume p is valid and properly aligned for reading a u32
    // and that there are at least 4 bytes available
    let actual_size = unsafe { u32::from_be(ptr::read_unaligned(p as *const u32)) };

    // Calculate the start of the data portion (after the u32)
    let data_ptr = unsafe { p.add(4) };

    let mut v = Vec::with_capacity(actual_size as usize);

    // SAFETY: We assume the memory from data_ptr to data_ptr+actual_size is valid
    unsafe {
        v.set_len(actual_size as usize);
        ptr::copy_nonoverlapping(
            data_ptr,
            v.as_mut_ptr(),
            actual_size as usize
        );
    }

    (actual_size, v)
}


impl ConvertibleContract for DriverExecutor {
    fn encode_input_fn(&self, fn_name: &str, json: &str) -> Result<Vec<u8>, Error> {
        let instance = self.instance.expect("instance should exist in executor");

        let fn_name_ptr = self.allocate_and_copy(fn_name.as_bytes(), fn_name.len())?;
        let json_ptr = self.allocate_and_copy(json.as_bytes(), json.len())?;
        const OUT_BUF_SIZE: usize = 65536;
        let out_ptr = self.allocate(OUT_BUF_SIZE)?;

        let f = instance.get_typed_func::<(*mut u8, usize, *mut u8, usize, *mut u8, usize), i32>(&mut self.store, "encode_input_fn")?;
        let _error_code = f.call(&mut self.store, (fn_name_ptr, fn_name.len(), json_ptr, json.len(), out_ptr, OUT_BUF_SIZE))?;
        // todo error_code

        self.deallocate(fn_name_ptr, fn_name.len());
        self.deallocate(json_ptr, json.len());

        let (_, out_vector) = read_u32_be_and_bytes(out_ptr);
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
