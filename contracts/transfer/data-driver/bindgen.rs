// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

mod driver;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use driver::ContractDriver;
use dusk_data_driver::ConvertibleContract;
use wasm_bindgen::prelude::*;

/// Set dlmalloc as the global allocator for heap allocs
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

/// Sends logs to the JS console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct WasmContractDriver {
    inner: ContractDriver,
}
impl Default for WasmContractDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmContractDriver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: ContractDriver,
        }
    }

    #[wasm_bindgen]
    pub fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, JsValue> {
        self.inner
            .encode_input_fn(fn_name, json)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.inner
            .decode_input_fn(fn_name, rkyv)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.inner
            .decode_output_fn(fn_name, rkyv)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.inner
            .decode_event(event_name, rkyv)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn get_schema(&self) -> String {
        self.inner.get_schema()
    }
}

fn main() {
    // Required for binaries, even if unused in WASM
}
