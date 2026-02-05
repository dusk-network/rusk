// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error-type for dusk-core.

use alloc::string::{String, ToString};
use core::fmt;

/// The dusk-core error type.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Rkyv serialization.
    Rkyv(String),
    /// Json serialization
    Json(String),
    /// Unsupported
    Unsupported(String),
    /// Other
    Other(String),
    /// WASM runtime error (reader feature only)
    #[cfg(feature = "reader")]
    WasmRuntime(String),
    /// WASM memory error (reader feature only)
    #[cfg(feature = "reader")]
    WasmMemory(String),
    /// WASM export error (reader feature only)
    #[cfg(feature = "reader")]
    WasmExport(String),
    /// FFI call error (reader feature only)
    #[cfg(feature = "reader")]
    FfiError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Data-Driver Error: {:?}", &self)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}
