// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use dusk_data_driver::ConvertibleContract;
use wasm_bindgen::prelude::*;

use super::ContractDriver;

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
#[derive(Default)]
pub struct WasmContractDriver(ContractDriver);

#[wasm_bindgen]
impl WasmContractDriver {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[wasm_bindgen]
    pub fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, JsValue> {
        self.0
            .encode_input_fn(fn_name, json)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.0
            .decode_input_fn(fn_name, rkyv)
            .map(|json| json.to_string())
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.0
            .decode_output_fn(fn_name, rkyv)
            .map(|json| json.to_string())
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<String, JsValue> {
        self.0
            .decode_event(event_name, rkyv)
            .map(|json| json.to_string())
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    }

    #[wasm_bindgen]
    pub fn get_schema(&self) -> String {
        self.0.get_schema()
    }
}
