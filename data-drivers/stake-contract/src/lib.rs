// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This crate provides a data driver implementation for dusk stake contract.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;

#[cfg(target_family = "wasm")]
mod bindgen;

use alloc::string::String;
use alloc::vec::Vec;

use dusk_data_driver::{ConvertibleContract, Error};

/// The contract driver for encoding and decoding transactions.
#[derive(Default)]
pub struct ContractDriver;

impl ConvertibleContract for ContractDriver {
    #[allow(unused_variables)]
    fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<serde_json::Value, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<serde_json::Value, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<serde_json::Value, Error> {
        todo!()
    }

    fn get_schema(&self) -> String {
        todo!()
    }
}
