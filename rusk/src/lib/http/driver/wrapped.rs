// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::ContractId;
use dusk_data_driver::{ConvertibleContract, Error};
use dusk_wasmtime::{self as w, AsContext, AsContextMut};
use serde_json::Value;
use std::{convert::TryInto, str};

use super::DriverExecutor;

pub struct WrappedDataDriver<'a> {
    pub(crate) exec: &'a DriverExecutor,
    pub(crate) id: ContractId,
}

impl<'a> WrappedDataDriver<'a> {
    // --- helpers to borrow store/instance mutably for this contract ---
    fn with_ctx<R>(
        &self,
        f: impl FnOnce(&mut w::Store<()>, &w::Instance) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let instance = self
            .exec
            .instances
            .get(&self.id)
            .ok_or_else(|| Error::Other("instance not found".into()))?;
        let mut store = self.exec.store.write().unwrap();

        f(&mut store, instance)
    }

    // --- wasm helpers ---
    fn get_memory(
        store: w::StoreContextMut<()>,
        instance: &w::Instance,
    ) -> Result<w::Memory, Error> {
        instance
            .get_memory(store, "memory")
            .ok_or_else(|| Error::Other("memory export not found".into()))
    }

    fn func_i32_i32(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
        name: &str,
    ) -> Result<w::TypedFunc<i32, i32>, Error> {
        let f = instance
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| Error::Other(format!("export {name} not found")))?;
        f.typed::<i32, i32>(store.as_context()).map_err(|e| {
            Error::Other(format!("{name} signature mismatch: {e}"))
        })
    }

    fn func_2_to_i32(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
        name: &str,
    ) -> Result<w::TypedFunc<(i32, i32), i32>, Error> {
        let f = instance
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| Error::Other(format!("export {name} not found")))?;
        f.typed::<(i32, i32), i32>(store.as_context()).map_err(|e| {
            Error::Other(format!("{name} signature mismatch: {e}"))
        })
    }

    fn func_6_to_i32(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
        name: &str,
    ) -> Result<w::TypedFunc<(i32, i32, i32, i32, i32, i32), i32>, Error> {
        let f = instance
            .get_func(store.as_context_mut(), name)
            .ok_or_else(|| Error::Other(format!("export {name} not found")))?;
        f.typed::<(i32, i32, i32, i32, i32, i32), i32>(store.as_context())
            .map_err(|e| {
                Error::Other(format!("{name} signature mismatch: {e}"))
            })
    }

    fn alloc_copy(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
        bytes: &[u8],
    ) -> Result<(i32, i32), Error> {
        let alloc =
            Self::func_i32_i32(store.as_context_mut(), instance, "alloc")?;
        let ptr = alloc
            .call(store.as_context_mut(), bytes.len() as i32)
            .map_err(|e| Error::Other(format!("alloc call failed: {e}")))?;
        let mem = Self::get_memory(store.as_context_mut(), instance)?;
        let data = mem.data_mut(store.as_context_mut());
        let start = ptr as usize;
        let end = start + bytes.len();
        data.get_mut(start..end)
            .ok_or_else(|| {
                Error::Other("alloc returned out-of-range ptr".into())
            })?
            .copy_from_slice(bytes);
        Ok((ptr, bytes.len() as i32))
    }

    fn dealloc(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
        ptr: i32,
        size: i32,
    ) -> Result<(), Error> {
        // (i32, i32) -> i32 ; ignore return
        let dealloc =
            Self::func_2_to_i32(store.as_context_mut(), instance, "dealloc")?;
        let _ = dealloc
            .call(store.as_context_mut(), (ptr, size))
            .map_err(|e| Error::Other(format!("dealloc call failed: {e}")))?;
        Ok(())
    }

    fn call_json_with_inputs(
        &self,
        export: &str,
        inputs: &[&[u8]],
    ) -> Result<Value, Error> {
        self.with_ctx(|store, instance| {
            // alloc/copy inputs
            let mut temp_ptrs: Vec<(i32, i32)> =
                Vec::with_capacity(inputs.len());
            for b in inputs {
                temp_ptrs.push(Self::alloc_copy(
                    store.as_context_mut(),
                    instance,
                    b,
                )?);
            }

            // output buffer
            let out_size = 64 * 1024;
            let out_ptr =
                Self::func_i32_i32(store.as_context_mut(), instance, "alloc")?
                    .call(store.as_context_mut(), out_size as i32)
                    .map_err(|e| {
                        Error::Other(format!("alloc(out) failed: {e}"))
                    })?;

            // dispatch based on arity
            let code = match inputs.len() {
                0 => {
                    // (out_ptr, out_size)
                    Self::func_2_to_i32(
                        store.as_context_mut(),
                        instance,
                        export,
                    )?
                    .call(store.as_context_mut(), (out_ptr, out_size as i32))
                }
                2 => {
                    let (p1, l1) = temp_ptrs[0];
                    let (p2, l2) = temp_ptrs[1];
                    // (p1,l1,p2,l2,out_ptr,out_size)
                    Self::func_6_to_i32(
                        store.as_context_mut(),
                        instance,
                        export,
                    )?
                    .call(
                        store.as_context_mut(),
                        (p1, l1, p2, l2, out_ptr, out_size as i32),
                    )
                }
                n => {
                    // Not supported by the current FFI; extend if you add more
                    // arities.
                    return Err(Error::Other(format!(
                        "unsupported arity {n} for export {export}"
                    )));
                }
            }
            .map_err(|e| Error::Other(format!("{export} call failed: {e}")))?;

            // free inputs
            for (ptr, len) in &temp_ptrs {
                let _ =
                    Self::dealloc(store.as_context_mut(), instance, *ptr, *len);
            }

            if code != 0 {
                let err = Self::get_last_error_string(
                    store.as_context_mut(),
                    instance,
                )?;
                let _ = Self::dealloc(
                    store.as_context_mut(),
                    instance,
                    out_ptr,
                    out_size as i32,
                );
                return Err(Error::Other(format!(
                    "{export} error {code}: {err}"
                )));
            }

            // read [len|data]
            let payload = {
                let mem = Self::get_memory(store.as_context_mut(), instance)?;
                let data = mem.data(store.as_context());
                let start = out_ptr as usize;
                let len_bytes: [u8; 4] =
                    data[start..start + 4].try_into().unwrap();
                let actual_len = u32::from_le_bytes(len_bytes) as usize;
                data[start + 4..start + 4 + actual_len].to_vec()
            };

            // free output
            let _ = Self::dealloc(
                store.as_context_mut(),
                instance,
                out_ptr,
                out_size as i32,
            );

            let s = str::from_utf8(&payload)
                .map_err(|e| Error::Other(format!("UTF-8 error: {e}")))?;
            serde_json::from_str(s)
                .map_err(|e| Error::Other(format!("JSON parse error: {e}")))
        })
    }

    fn call_bytes_with_inputs(
        &self,
        export: &str,
        inputs: &[&[u8]],
    ) -> Result<Vec<u8>, Error> {
        self.with_ctx(|store, instance| {
            let mut temp_ptrs: Vec<(i32, i32)> =
                Vec::with_capacity(inputs.len());
            for b in inputs {
                temp_ptrs.push(Self::alloc_copy(
                    store.as_context_mut(),
                    instance,
                    b,
                )?);
            }

            let out_size = 64 * 1024;
            let out_ptr =
                Self::func_i32_i32(store.as_context_mut(), instance, "alloc")?
                    .call(store.as_context_mut(), out_size as i32)
                    .map_err(|e| {
                        Error::Other(format!("alloc(out) failed: {e}"))
                    })?;

            let code = match inputs.len() {
                0 => Self::func_2_to_i32(
                    store.as_context_mut(),
                    instance,
                    export,
                )?
                .call(store.as_context_mut(), (out_ptr, out_size as i32)),
                2 => {
                    let (p1, l1) = temp_ptrs[0];
                    let (p2, l2) = temp_ptrs[1];
                    Self::func_6_to_i32(
                        store.as_context_mut(),
                        instance,
                        export,
                    )?
                    .call(
                        store.as_context_mut(),
                        (p1, l1, p2, l2, out_ptr, out_size as i32),
                    )
                }
                n => {
                    return Err(Error::Other(format!(
                        "unsupported arity {n} for export {export}"
                    )));
                }
            }
            .map_err(|e| Error::Other(format!("{export} call failed: {e}")))?;

            // free inputs
            for (ptr, len) in &temp_ptrs {
                let _ =
                    Self::dealloc(store.as_context_mut(), instance, *ptr, *len);
            }

            if code != 0 {
                let err = Self::get_last_error_string(
                    store.as_context_mut(),
                    instance,
                )?;
                let _ = Self::dealloc(
                    store.as_context_mut(),
                    instance,
                    out_ptr,
                    out_size as i32,
                );
                return Err(Error::Other(format!(
                    "{export} error {code}: {err}"
                )));
            }

            let mem = Self::get_memory(store.as_context_mut(), instance)?;
            let data = mem.data(store.as_context());
            let start = out_ptr as usize;
            let len_bytes: [u8; 4] = data[start..start + 4].try_into().unwrap();
            let actual_len = u32::from_le_bytes(len_bytes) as usize;
            let vec = data[start + 4..start + 4 + actual_len].to_vec();

            let _ = Self::dealloc(
                store.as_context_mut(),
                instance,
                out_ptr,
                out_size as i32,
            );
            Ok(vec)
        })
    }

    fn get_last_error_string(
        mut store: w::StoreContextMut<()>,
        instance: &w::Instance,
    ) -> Result<String, Error> {
        let out_size = 1024;
        let out_ptr =
            Self::func_i32_i32(store.as_context_mut(), instance, "alloc")?
                .call(store.as_context_mut(), out_size as i32)
                .map_err(|e| Error::Other(format!("alloc(out) failed: {e}")))?;

        // (i32, i32) -> i32
        let get_last_error = Self::func_2_to_i32(
            store.as_context_mut(),
            instance,
            "get_last_error",
        )?;
        let _ = get_last_error
            .call(store.as_context_mut(), (out_ptr, out_size as i32))
            .map_err(|e| Error::Other(format!("get_last_error failed: {e}")))?;

        let mem = Self::get_memory(store.as_context_mut(), instance)?;
        let data = mem.data(store.as_context());
        let start = out_ptr as usize;
        let len_bytes: [u8; 4] = data[start..start + 4].try_into().unwrap();
        let actual_len = u32::from_le_bytes(len_bytes) as usize;
        let s =
            String::from_utf8_lossy(&data[start + 4..start + 4 + actual_len])
                .to_string();

        // free buffer
        let _ = Self::dealloc(
            store.as_context_mut(),
            instance,
            out_ptr,
            out_size as i32,
        );
        Ok(s)
    }
}

impl<'a> ConvertibleContract for WrappedDataDriver<'a> {
    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<Value, Error> {
        self.call_json_with_inputs(
            "decode_event",
            &[event_name.as_bytes(), rkyv],
        )
    }

    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<Value, Error> {
        self.call_json_with_inputs(
            "decode_input_fn",
            &[fn_name.as_bytes(), rkyv],
        )
    }

    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<Value, Error> {
        self.call_json_with_inputs(
            "decode_output_fn",
            &[fn_name.as_bytes(), rkyv],
        )
    }

    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        self.call_bytes_with_inputs(
            "encode_input_fn",
            &[fn_name.as_bytes(), json.as_bytes()],
        )
    }

    fn get_schema(&self) -> String {
        match self.call_json_with_inputs("get_schema", &[]) {
            Ok(Value::String(s)) => s,
            Ok(v) => v.to_string(),
            Err(_) => String::new(),
        }
    }

    fn get_version(&self) -> &'static str {
        match self.call_json_with_inputs("get_version", &[]) {
            Ok(Value::String(s)) => Box::leak(s.into_boxed_str()),
            Ok(v) => Box::leak(v.to_string().into_boxed_str()),
            Err(_) => "unknown",
        }
    }
}
