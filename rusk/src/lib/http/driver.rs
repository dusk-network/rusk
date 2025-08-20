// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::DerefMut;
use std::sync::Arc;

use dusk_wasmtime::{Config, Engine, Instance, Module, Store};
use parking_lot::RwLock;

use dusk_core::abi::ContractId;
use dusk_data_driver::{ConvertibleContract, Error, JsonValue};

fn config() -> Config {
    let mut config = Config::new();
    config.macos_use_mach_ports(false);
    config
}

const OUT_BUF_SIZE: usize = 65536;

#[derive(Clone, Debug)]
pub struct DriverExecutor {
    store: Arc<RwLock<Store<()>>>,
    instance: Option<Instance>,
    _contract_id: ContractId,
}

impl DriverExecutor {
    pub fn new() -> Self {
        let config = config();
        let engine = Engine::new(&config)
            .expect("Wasmtime engine configuration should be valid");
        let store = Store::<()>::new(&engine, ());
        Self {
            store: Arc::new(RwLock::new(store)),
            instance: None,
            _contract_id: ContractId::from_bytes([0u8; 32]),
        }
    }

    pub fn load_bytecode(
        &mut self,
        contract_id: &ContractId,
        bytecode: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let mut store = self.store.write();
        let store = store.deref_mut();
        let module = Module::new(store.engine(), bytecode.as_ref())?;
        let instance = Instance::new(store, &module, &[])?;
        self.instance = Some(instance);
        self._contract_id = *contract_id;
        Ok(())
    }

    pub fn init(&self) -> anyhow::Result<()> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let store = store.deref_mut();
        let init = instance.get_typed_func::<(), ()>(&mut *store, "init")?;
        // .map_err(|e| Error::Other(format!("init failed: {e}")))?;
        init.call(store, ())?;
        // .map_err(|e| Error::Other(format!("init failed: {e}")))?;
        Ok(())
    }

    fn allocate(&self, sz: usize) -> Result<*mut u8, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let store = store.deref_mut();
        let alloc =
            instance
                .get_typed_func::<i32, i32>(&mut *store, "alloc")
                .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        let mem = alloc
            .call(store, sz as i32)
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        Ok(mem as *mut u8)
    }

    fn allocate_and_copy(
        &self,
        bytes: &[u8],
        sz: usize,
    ) -> Result<*mut u8, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let wasm_memory = {
            let mut store = self.store.write();
            let mut store = store.deref_mut();
            instance
                .get_memory(&mut store, "memory")
                .ok_or(Error::Other(format!("getting memory failed")))?
        };
        let mem = self.allocate(sz)?;
        let mut store = self.store.write();
        let store = store.deref_mut();
        wasm_memory
            .write(store, mem as usize, bytes)
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        Ok(mem)
    }

    fn deallocate(&self, ptr: *mut u8, sz: usize) -> Result<(), Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let mut store = store.deref_mut();
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        dealloc
            .call(&mut store, (ptr as i32, sz as i32))
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        Ok(())
    }

    // reads from a given memory pointer
    // assumes first 4 bytes hold Big Endian-encoded buffer length, say,
    // 'actual_size' having obtained 'actual_size' in this way, function assumes
    // that the subsequent buffer bytes contain 'actual_size' bytes
    // the bytes are then copied into a vector and returned
    fn read_u32_be_and_bytes(&self, p: *const u8) -> Result<Vec<u8>, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let mut store = store.deref_mut();
        let wasm_memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or(Error::Other(format!("getting memory failed")))?;

        // SAFETY: We assume p is valid and properly aligned for reading a u32
        // and that there are at least 4 bytes available
        let mut actual_size_buf = [0u8; 4];
        wasm_memory
            .read(&mut store, p as usize, &mut actual_size_buf)
            .map_err(|e| {
                Error::Other(format!("reading wasm memory failed: {e}"))
            })?;
        let actual_size = u32::from_le_bytes(actual_size_buf);

        // Calculate the start of the data portion (after the u32)
        let data_ptr = unsafe { p.add(4) };

        let mut buffer = vec![0u8; actual_size as usize];
        wasm_memory
            .read(store, data_ptr as usize, &mut buffer)
            .map_err(|e| {
                Error::Other(format!("reading wasm memory failed: {e}"))
            })?;

        Ok(buffer)
    }

    fn decoding_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
        decoding_fn_name: &str,
    ) -> Result<JsonValue, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");

        let fn_name_ptr =
            self.allocate_and_copy(fn_name.as_bytes(), fn_name.len())?;
        let rkyv_ptr = self.allocate_and_copy(rkyv, rkyv.len())?;
        let out_ptr = self.allocate(OUT_BUF_SIZE)?;

        let error_code = {
            let mut store = self.store.write();
            let mut store = store.deref_mut();
            let f = instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
                    &mut *store,
                    decoding_fn_name,
                )
                .map_err(|e| {
                    Error::Other(format!("{decoding_fn_name} failed: {e}"))
                })?;
            f.call(
                &mut store,
                (
                    fn_name_ptr as i32,
                    fn_name.len() as i32,
                    rkyv_ptr as i32,
                    rkyv.len() as i32,
                    out_ptr as i32,
                    OUT_BUF_SIZE as i32,
                ),
            )
            .map_err(|e| {
                Error::Other(format!("{decoding_fn_name} failed: {e}"))
            })?
        };

        self.deallocate(fn_name_ptr, fn_name.len())?;
        self.deallocate(rkyv_ptr, rkyv.len())?;

        let out_vector = self.read_u32_be_and_bytes(out_ptr)?;
        self.deallocate(out_ptr, OUT_BUF_SIZE)?;
        match error_code {
            0 => Ok(serde_json::from_slice(&out_vector)?),
            _ => Err(Error::Other(format!(
                "{decoding_fn_name} failed with: {error_code}"
            ))),
        }
    }
}

impl ConvertibleContract for DriverExecutor {
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");

        let fn_name_ptr =
            self.allocate_and_copy(fn_name.as_bytes(), fn_name.len())?;
        let json_ptr = self.allocate_and_copy(json.as_bytes(), json.len())?;
        let out_ptr = self.allocate(OUT_BUF_SIZE)?;

        let error_code = {
            let mut store = self.store.write();
            let mut store = store.deref_mut();
            let f = instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
                    &mut *store,
                    "encode_input_fn",
                )
                .map_err(|e| {
                    Error::Other(format!("encode_input_fn failed: {e}"))
                })?;
            f.call(
                &mut store,
                (
                    fn_name_ptr as i32,
                    fn_name.len() as i32,
                    json_ptr as i32,
                    json.len() as i32,
                    out_ptr as i32,
                    OUT_BUF_SIZE as i32,
                ),
            )
            .map_err(|e| Error::Other(format!("encode_input_fn failed: {e}")))?
        };
        println!("444");

        self.deallocate(fn_name_ptr, fn_name.len())?;
        self.deallocate(json_ptr, json.len())?;
        println!("555");

        let out_vector = self.read_u32_be_and_bytes(out_ptr)?;
        self.deallocate(out_ptr, OUT_BUF_SIZE)?;
        println!("666");
        match error_code {
            0 => Ok(out_vector),
            _ => Err(Error::Other(format!(
                "encode_input_fn failed with: {error_code}"
            ))),
        }
    }

    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decoding_fn(fn_name, rkyv, "decode_input_fn")
    }

    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decoding_fn(fn_name, rkyv, "decode_output_fn")
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decoding_fn(event_name, rkyv, "decode_event")
    }

    fn get_schema(&self) -> String {
        "".to_string()
    }
}
