// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::DerefMut;
use std::ptr;
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

    fn allocate(&self, sz: usize) -> Result<*mut u8, Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let store = store.deref_mut();
        let alloc =
            instance
                .get_typed_func::<u32, u64>(&mut *store, "alloc")
                .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        let mem = alloc
            .call(store, sz as u32)
            .map_err(|e| Error::Other(format!("allocate failed: {e}")))?;
        Ok(mem as *mut u8)
    }

    fn allocate_and_copy(
        &self,
        bytes: &[u8],
        sz: usize,
    ) -> Result<*mut u8, Error> {
        let mem = self.allocate(sz)?;
        let dst_slice = unsafe { std::slice::from_raw_parts_mut(mem, sz) };
        dst_slice.copy_from_slice(&bytes[..sz]);
        Ok(mem)
    }

    fn deallocate(&self, ptr: *mut u8, sz: usize) -> Result<(), Error> {
        let instance =
            self.instance.expect("instance should exist in executor");
        let mut store = self.store.write();
        let mut store = store.deref_mut();
        let dealloc = instance
            .get_typed_func::<(u64, u32), ()>(&mut store, "dealloc")
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        dealloc
            .call(&mut store, (ptr as u64, sz as u32))
            .map_err(|e| Error::Other(format!("deallocate failed: {e}")))?;
        Ok(())
    }
}

// reads from a given memory pointer
// assumes first 4 bytes hold Big Endian-encoded buffer length, say,
// 'actual_size' having obtained 'actual_size' in this way, function assumes
// that the subsequent buffer bytes contain 'actual_size' bytes
// the bytes are then copied into a vector and returned
fn read_u32_be_and_bytes(p: *const u8) -> Vec<u8> {
    // SAFETY: We assume p is valid and properly aligned for reading a u32
    // and that there are at least 4 bytes available
    let actual_size =
        unsafe { u32::from_be(ptr::read_unaligned(p as *const u32)) };

    // Calculate the start of the data portion (after the u32)
    let data_ptr = unsafe { p.add(4) };

    let mut v = Vec::with_capacity(actual_size as usize);

    // SAFETY: We assume the memory from data_ptr to data_ptr+actual_size is
    // valid
    unsafe {
        v.set_len(actual_size as usize);
        ptr::copy_nonoverlapping(
            data_ptr,
            v.as_mut_ptr(),
            actual_size as usize,
        );
    }

    v
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

        let mut store = self.store.write();
        let mut store = store.deref_mut();
        let f = instance
            .get_typed_func::<(u64, u32, u64, u32, u64, u32), u32>(
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
                    fn_name_ptr as u64,
                    fn_name.len() as u32,
                    json_ptr as u64,
                    json.len() as u32,
                    out_ptr as u64,
                    OUT_BUF_SIZE as u32,
                ),
            )
            .map_err(|e| {
                Error::Other(format!("encode_input_fn failed: {e}"))
            })?;

        self.deallocate(fn_name_ptr, fn_name.len())?;
        self.deallocate(json_ptr, json.len())?;

        let out_vector = read_u32_be_and_bytes(out_ptr);
        self.deallocate(out_ptr, OUT_BUF_SIZE)?;
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
        let instance =
            self.instance.expect("instance should exist in executor");

        let fn_name_ptr =
            self.allocate_and_copy(fn_name.as_bytes(), fn_name.len())?;
        let rkyv_ptr = self.allocate_and_copy(rkyv, rkyv.len())?;
        let out_ptr = self.allocate(OUT_BUF_SIZE)?;

        let mut store = self.store.write();
        let mut store = store.deref_mut();
        let f = instance
            .get_typed_func::<(u64, u32, u64, u32, u64, u32), u32>(
                &mut *store,
                "decode_input_fn",
            )
            .map_err(|e| {
                Error::Other(format!("decode_input_fn failed: {e}"))
            })?;
        let error_code = f
            .call(
                &mut store,
                (
                    fn_name_ptr as u64,
                    fn_name.len() as u32,
                    rkyv_ptr as u64,
                    rkyv.len() as u32,
                    out_ptr as u64,
                    OUT_BUF_SIZE as u32,
                ),
            )
            .map_err(|e| {
                Error::Other(format!("decode_input_fn failed: {e}"))
            })?;

        self.deallocate(fn_name_ptr, fn_name.len())?;
        self.deallocate(rkyv_ptr, rkyv.len())?;

        let out_vector = read_u32_be_and_bytes(out_ptr);
        self.deallocate(out_ptr, OUT_BUF_SIZE)?;
        match error_code {
            0 => {
                let v = serde_json::from_slice(&out_vector)?;
                Ok(v)
            }
            _ => Err(Error::Other(format!(
                "decode_input_fn failed with: {error_code}"
            ))),
        }
    }

    fn decode_output_fn(
        &self,
        _fn_name: &str,
        _rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        Ok(JsonValue::Null)
    }

    fn decode_event(
        &self,
        _event_name: &str,
        _rkyv: &[u8],
    ) -> Result<JsonValue, Error> {
        Ok(JsonValue::Null)
    }

    fn get_schema(&self) -> String {
        "".to_string()
    }
}
