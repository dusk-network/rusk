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
    instance: Instance,
    contract_id: ContractId,
}

impl DriverExecutor {
    pub fn from_bytecode(
        contract_id: &ContractId,
        bytecode: impl AsRef<[u8]>,
    ) -> anyhow::Result<Self> {
        let config = config();
        let engine = Engine::new(&config)
            .expect("Wasmtime engine configuration should be valid");
        let mut store = Store::<()>::new(&engine, ());
        let module = Module::new(store.engine(), bytecode.as_ref())?;
        let instance = Instance::new(&mut store, &module, &[])?;
        Ok(Self {
            store: Arc::new(RwLock::new(store)),
            instance,
            contract_id: *contract_id,
        })
    }

    pub fn init(&self) -> anyhow::Result<()> {
        let mut store = self.store.write();
        let store = store.deref_mut();
        let init = self
            .instance
            .get_typed_func::<(), ()>(&mut *store, "init")?;
        init.call(store, ())?;
        Ok(())
    }

    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }

    fn allocate(
        &self,
        store: &mut Store<()>,
        sz: usize,
    ) -> Result<*mut u8, Error> {
        let alloc = self
            .instance
            .get_typed_func::<i32, i32>(&mut *store, "alloc")
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        let mem = alloc
            .call(&mut *store, sz as i32)
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        Ok(mem as *mut u8)
    }

    fn allocate_and_copy(
        &self,
        store: &mut Store<()>,
        bytes: &[u8],
        sz: usize,
    ) -> Result<*mut u8, Error> {
        let mem = self.allocate(&mut *store, sz)?;
        let wasm_memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .ok_or(Error::Other(format!("getting memory failed")))?;
        wasm_memory
            .write(&mut *store, mem as usize, bytes)
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        Ok(mem)
    }

    fn deallocate(
        &self,
        store: &mut Store<()>,
        ptr: *mut u8,
        sz: usize,
    ) -> Result<(), Error> {
        let dealloc = self
            .instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc")
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        dealloc
            .call(&mut *store, (ptr as i32, sz as i32))
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        Ok(())
    }

    // Reads bytes from a given memory pointer assuming that
    // first 4 bytes hold little endian-encoded buffer length L.
    // Subsequent bytes form a buffer of length L
    //
    // Returns: vector containing copy of the buffer
    fn read_u32_le_and_bytes(
        &self,
        store: &mut Store<()>,
        p: *const u8,
    ) -> Result<Vec<u8>, Error> {
        let wasm_memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .ok_or(Error::Other(format!("getting memory failed")))?;

        let mut buf_len_buf = [0u8; 4];
        wasm_memory
            .read(&mut *store, p as usize, &mut buf_len_buf)
            .map_err(|e| {
                Error::Other(format!("reading wasm memory failed: {e}"))
            })?;
        let buf_len = u32::from_le_bytes(buf_len_buf);

        let data_ptr = unsafe { p.add(4) };

        let mut buffer = vec![0u8; buf_len as usize];
        wasm_memory
            .read(&mut *store, data_ptr as usize, &mut buffer)
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
        let mut store = self.store.write();
        let mut store = store.deref_mut();

        let fn_name_ptr = self.allocate_and_copy(
            &mut *store,
            fn_name.as_bytes(),
            fn_name.len(),
        )?;
        let rkyv_ptr = self.allocate_and_copy(&mut *store, rkyv, rkyv.len())?;
        let out_ptr = self.allocate(&mut *store, OUT_BUF_SIZE)?;

        let f = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
                &mut *store,
                decoding_fn_name,
            )
            .map_err(|e| {
                Error::Other(format!("{decoding_fn_name} failed: {e}"))
            })?;
        let error_code = f
            .call(
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
            })?;

        self.deallocate(&mut store, fn_name_ptr, fn_name.len())?;
        self.deallocate(&mut store, rkyv_ptr, rkyv.len())?;
        let out_vector = self.read_u32_le_and_bytes(&mut *store, out_ptr)?;
        self.deallocate(&mut store, out_ptr, OUT_BUF_SIZE)?;
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
        let mut store = self.store.write();
        let mut store = store.deref_mut();

        let fn_name_ptr = self.allocate_and_copy(
            &mut *store,
            fn_name.as_bytes(),
            fn_name.len(),
        )?;
        let json_ptr =
            self.allocate_and_copy(&mut *store, json.as_bytes(), json.len())?;
        let out_ptr = self.allocate(&mut *store, OUT_BUF_SIZE)?;

        let f = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
                &mut *store,
                "encode_input_fn",
            )
            .map_err(|e| {
                Error::Other(format!("encode_input_fn failed: {e}"))
            })?;
        let error_code = f
            .call(
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
            .map_err(|e| {
                Error::Other(format!("encode_input_fn failed: {e}"))
            })?;

        self.deallocate(&mut store, fn_name_ptr, fn_name.len())?;
        self.deallocate(&mut store, json_ptr, json.len())?;
        let out_vector = self.read_u32_le_and_bytes(&mut *store, out_ptr)?;
        self.deallocate(&mut store, out_ptr, OUT_BUF_SIZE)?;
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
