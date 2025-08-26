// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use hyper::HeaderMap;
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone)]
pub struct DriverStoreConfig {
    pub driver_store_path: Option<PathBuf>,
    pub driver_store_limit: u64,
}

impl Default for DriverStoreConfig {
    fn default() -> Self {
        Self {
            driver_store_path: None,
            driver_store_limit: default_driver_store_limit(),
        }
    }
}

const fn default_driver_store_limit() -> u64 {
    1024
}
