// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use dusk_core::transfer::{MoonlightTransactionEvent, MOONLIGHT_TOPIC};
use dusk_data_driver::{rkyv_to_json, ConvertibleContract};

/// JSON schema definition for contract data
const SCHEMA: &str = r#"
{
    "type": "object",
    "properties": {
        "recipient": { "type": "string" },
        "amount": { "type": "integer" }
    },
    "required": ["recipient", "amount"]
}
"#;
use dusk_data_driver::Error;

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
    ) -> Result<String, Error> {
        todo!()
    }

    #[allow(unused_variables)]
    fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv: &[u8],
    ) -> Result<String, Error> {
        todo!()
    }

    fn decode_event(
        &self,
        event_name: &str,
        rkyv: &[u8],
    ) -> Result<String, Error> {
        match event_name {
            MOONLIGHT_TOPIC => rkyv_to_json::<MoonlightTransactionEvent>(rkyv),
            _ => todo!(),
        }
    }

    fn get_schema(&self) -> String {
        SCHEMA.to_string()
    }
}
