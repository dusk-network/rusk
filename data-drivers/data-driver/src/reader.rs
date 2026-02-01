// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Data-Driver WASM Reader
//!
//! This module provides a thread-safe Rust API for loading and interacting
//! with compiled data-driver WASM modules.
//!
//! ## Features
//!
//! - Load compiled data-driver WASM blobs
//! - Encode JSON inputs to RKYV bytes
//! - Decode RKYV bytes (inputs, outputs, events) to JSON
//! - Access contract metadata (schema, version)
//! - Safe memory management with automatic cleanup
//! - Thread-safe: can be shared across threads via `Arc<RwLock<>>`
//! - Clone support for easy sharing
//!
//! ## Usage
//!
//! ```no_run
//! use dusk_data_driver::reader::DriverReader;
//! use dusk_data_driver::Error;
//!
//! # fn main() -> Result<(), Error> {
//! // Load compiled WASM driver and automatically call init()
//! let wasm_bytes = std::fs::read("stake_contract_driver.wasm")
//!     .map_err(|e| Error::Other(e.to_string()))?;
//! let driver = DriverReader::new(&wasm_bytes)?;
//!
//! // Get metadata
//! let version = driver.get_version()?;
//! println!("Driver version: {}", version);
//!
//! // Encode input
//! let json_input = r#"{"amount": "100"}"#;
//! let rkyv_bytes = driver.encode_input_fn("stake", json_input)?;
//!
//! // Decode output
//! let json_output = driver.decode_output_fn("get_stake", &rkyv_bytes)?;
//! println!("Result: {}", json_output);
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flag
//!
//! This module is only available when the `reader` feature is enabled:

use std::fmt::Debug;
use std::sync::Arc;

use dusk_core::abi::ContractId;
use dusk_wasmtime::{Engine, Instance, Memory, Module, Store, TypedFunc};
use parking_lot::RwLock;

use crate::{Error, JsonValue};

/// Default output buffer size (64 KB)
const OUTPUT_BUFFER_SIZE: i32 = 64 * 1024;

/// Error buffer size (1 KB)
/// This is usually sufficient for an enum + error message.
const ERROR_BUFFER_SIZE: i32 = 1024;

/// Type alias for the common 6-arg input tuple used in codec FFI functions
type CodecInputTuple = (i32, i32, i32, i32, i32, i32);

/// Type alias for the common 6-arg FFI function signature
/// For encode: (`fn_ptr`, `fn_len`, `json_ptr`, `json_len`, `out_ptr`,
/// `out_size`) -> i32 For decode: (`fn_ptr`, `fn_len`, `rkyv_ptr`, `rkyv_len`,
/// `out_ptr`, `out_size`) -> i32
type CodecFfiFunc = TypedFunc<CodecInputTuple, i32>;

/// Type alias for simple getter functions
/// For example: (`out_ptr`, `out_size`) -> i32
type GetterFfiFunc = TypedFunc<(i32, i32), i32>;

/// Internal state that requires mutable access
struct ReaderInner {
    /// Wasmtime store containing execution state
    store: Store<()>,
    /// Reference to WASM linear memory
    memory: Memory,
    /// alloc(len: i32) -> i32
    alloc_fn: TypedFunc<i32, i32>,
    /// dealloc(ptr: i32, len: i32) -> ()
    dealloc_fn: TypedFunc<(i32, i32), ()>,
    /// `encode_input_fn`
    encode_input_fn_export: CodecFfiFunc,
    /// `decode_input_fn`
    decode_input_fn_export: CodecFfiFunc,
    /// `decode_output_fn`
    decode_output_fn_export: CodecFfiFunc,
    /// `decode_event`
    decode_event_fn_export: CodecFfiFunc,
    /// get schema(`out_ptr`, `out_size`) -> i32
    get_schema_fn: GetterFfiFunc,
    /// get version(`out_ptr`, `out_size`) -> i32
    get_version_fn: GetterFfiFunc,
    /// get last error(`out_ptr`, `out_size`) -> i32
    get_last_error_fn: GetterFfiFunc,
}

/// A thread-safe reader for data-driver WASM modules.
///
/// This struct provides a high-level API for interacting with compiled
/// data-driver WASM blobs, enabling encoding/decoding of contract data
/// from Rust applications.
#[derive(Clone)]
pub struct DriverReader {
    /// Thread-safe inner state
    inner: Arc<RwLock<ReaderInner>>,
    /// Associated contract ID
    contract_id: ContractId,
    /// WASM instance (kept alive for the lifetime of the reader)
    _instance: Arc<Instance>,
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for DriverReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverReader")
            .field("contract_id", &self.contract_id)
            .finish()
    }
}

impl DriverReader {
    /// Creates a new driver reader from WASM bytes.
    ///
    /// This method loads and validates a compiled data-driver WASM module,
    /// caching all required function exports for efficient repeated calls.
    /// The driver's `init()` function is automatically called during
    /// construction.
    ///
    /// # Parameters
    ///
    /// - `wasm_bytes`: The compiled WASM binary data
    ///
    /// # Returns
    ///
    /// - `Ok(DriverReader)`: Successfully loaded and initialized driver
    /// - `Err(Error)`: If loading or initialization fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmRuntime`: Invalid WASM bytes or compilation failure
    /// - `Error::WasmExport`: Missing required exports or wrong signatures
    /// - `Error::FfiError`: Driver initialization failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dusk_data_driver::reader::DriverReader;
    /// use dusk_data_driver::Error;
    ///
    /// # fn main() -> Result<(), Error> {
    /// let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// let mut driver = DriverReader::new(&wasm_bytes)?;
    /// // Driver is now ready to use.
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, Error> {
        Self::with_contract_id(wasm_bytes, ContractId::from_bytes([0u8; 32]))
    }

    /// Creates a new driver reader from WASM bytes with an associated contract
    /// ID.
    ///
    /// Same as [`new`](Self::new), but associates a contract ID with the
    /// reader for identification purposes.
    ///
    /// # Parameters
    ///
    /// - `wasm_bytes`: The compiled WASM binary data
    /// - `contract_id`: The contract identifier
    ///
    /// # Returns
    ///
    /// - `Ok(DriverReader)`: Successfully loaded and initialized driver
    /// - `Err(Error)`: If loading or initialization fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmRuntime`: Invalid WASM bytes or compilation failure
    /// - `Error::WasmExport`: Missing required exports or wrong signatures
    /// - `Error::FfiError`: Driver initialization failed
    #[allow(clippy::too_many_lines)]
    pub fn with_contract_id(
        wasm_bytes: &[u8],
        contract_id: ContractId,
    ) -> Result<Self, Error> {
        // Create Wasmtime engine with default configuration
        let engine = Engine::default();

        // Compile the WASM module
        let module = Module::from_binary(&engine, wasm_bytes).map_err(|e| {
            Error::WasmRuntime(format!("Failed to compile WASM: {e}"))
        })?;

        // Create store. Used for keeping instance state.
        let mut store = Store::new(&engine, ());

        // Instantiate with empty imports
        let instance =
            Instance::new(&mut store, &module, &[]).map_err(|e| {
                Error::WasmRuntime(format!("Failed to instantiate WASM: {e}"))
            })?;

        // Get memory export
        let memory =
            instance.get_memory(&mut store, "memory").ok_or_else(|| {
                Error::WasmExport("Missing 'memory' export".into())
            })?;

        // Get and cache all required function exports
        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'alloc' export: {e}"))
            })?;

        let dealloc_fn = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'dealloc' export: {e}"))
            })?;

        let encode_input_fn_export = instance
            .get_typed_func::<CodecInputTuple, i32>(
                &mut store,
                "encode_input_fn",
            )
            .map_err(|e| {
                Error::WasmExport(format!(
                    "Invalid 'encode_input_fn' export: {e}"
                ))
            })?;

        let decode_input_fn_export = instance
            .get_typed_func::<CodecInputTuple, i32>(
                &mut store,
                "decode_input_fn",
            )
            .map_err(|e| {
                Error::WasmExport(format!(
                    "Invalid 'decode_input_fn' export: {e}"
                ))
            })?;

        let decode_output_fn_export = instance
            .get_typed_func::<CodecInputTuple, i32>(
                &mut store,
                "decode_output_fn",
            )
            .map_err(|e| {
                Error::WasmExport(format!(
                    "Invalid 'decode_output_fn' export: {e}"
                ))
            })?;

        let decode_event_fn_export = instance
            .get_typed_func::<CodecInputTuple, i32>(&mut store, "decode_event")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'decode_event' export: {e}"))
            })?;

        let get_schema_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "get_schema")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'get_schema' export: {e}"))
            })?;

        let get_version_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "get_version")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'get_version' export: {e}"))
            })?;

        let get_last_error_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "get_last_error")
            .map_err(|e| {
                Error::WasmExport(format!(
                    "Invalid 'get_last_error' export: {e}"
                ))
            })?;

        // Get init function (required for proper driver initialization)
        let init_fn = instance
            .get_typed_func::<(), ()>(&mut store, "init")
            .map_err(|e| {
                Error::WasmExport(format!("Invalid 'init' export: {e}"))
            })?;

        // Call init immediately to initialize the contract driver
        // The init function must be called before using any FFI functions
        init_fn
            .call(&mut store, ())
            .map_err(|e| Error::FfiError(format!("init() failed: {e}")))?;

        let inner = ReaderInner {
            store,
            memory,
            alloc_fn,
            dealloc_fn,
            encode_input_fn_export,
            decode_input_fn_export,
            decode_output_fn_export,
            decode_event_fn_export,
            get_schema_fn,
            get_version_fn,
            get_last_error_fn,
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
            contract_id,
            _instance: Arc::new(instance),
        })
    }

    /// Returns the associated contract ID.
    #[must_use]
    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }

    /// Encodes JSON input to RKYV bytes for a contract function.
    ///
    /// # Parameters
    ///
    /// - `fn_name`: Name of the contract function
    /// - `json`: JSON string representing the function input
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<u8>)`: RKYV-encoded bytes ready for contract call
    /// - `Err(Error)`: If encoding fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Encoding failed (invalid JSON or unsupported
    ///   function)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// let json = r#"{"amount": "100"}"#;
    /// let rkyv_bytes = driver.encode_input_fn("stake", json)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn encode_input_fn(
        &self,
        fn_name: &str,
        json: &str,
    ) -> Result<Vec<u8>, Error> {
        self.call_with_name_and_data(
            fn_name.as_bytes(),
            json.as_bytes(),
            |inner, args| {
                inner.encode_input_fn_export.call(&mut inner.store, args)
            },
            "encode_input_fn",
        )
    }

    /// Decodes RKYV input bytes to JSON for a contract function.
    ///
    /// # Parameters
    ///
    /// - `fn_name`: Name of the contract function
    /// - `rkyv_bytes`: RKYV-encoded input bytes
    ///
    /// # Returns
    ///
    /// - `Ok(JsonValue)`: Decoded JSON representation
    /// - `Err(Error)`: If decoding fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Decoding failed (invalid RKYV or unsupported
    ///   function)
    /// - `Error::Json`: JSON parsing failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// # let rkyv_bytes = vec![0u8; 32];
    /// let json = driver.decode_input_fn("stake", &rkyv_bytes)?;
    /// println!("{}", json);
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode_input_fn(
        &self,
        fn_name: &str,
        rkyv_bytes: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decode_common(
            fn_name.as_bytes(),
            rkyv_bytes,
            |inner, args| {
                inner.decode_input_fn_export.call(&mut inner.store, args)
            },
            "decode_input_fn",
        )
    }

    /// Decodes RKYV output bytes to JSON for a contract function.
    ///
    /// # Parameters
    ///
    /// - `fn_name`: Name of the contract function
    /// - `rkyv_bytes`: RKYV-encoded output bytes
    ///
    /// # Returns
    ///
    /// - `Ok(JsonValue)`: Decoded JSON representation
    /// - `Err(Error)`: If decoding fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Decoding failed (invalid RKYV or unsupported
    ///   function)
    /// - `Error::Json`: JSON parsing failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// # let rkyv_bytes = vec![0u8; 32];
    /// let json = driver.decode_output_fn("get_stake", &rkyv_bytes)?;
    /// println!("{}", json);
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode_output_fn(
        &self,
        fn_name: &str,
        rkyv_bytes: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decode_common(
            fn_name.as_bytes(),
            rkyv_bytes,
            |inner, args| {
                inner.decode_output_fn_export.call(&mut inner.store, args)
            },
            "decode_output_fn",
        )
    }

    /// Decodes RKYV event bytes to JSON.
    ///
    /// # Parameters
    ///
    /// - `event_name`: Name or topic of the event
    /// - `rkyv_bytes`: RKYV-encoded event bytes
    ///
    /// # Returns
    ///
    /// - `Ok(JsonValue)`: Decoded JSON representation
    /// - `Err(Error)`: If decoding fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Decoding failed (invalid RKYV or unsupported event)
    /// - `Error::Json`: JSON parsing failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// # let rkyv_bytes = vec![0u8; 32];
    /// let json = driver.decode_event("stake", &rkyv_bytes)?;
    /// # println!("{}", json);
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode_event(
        &self,
        event_name: &str,
        rkyv_bytes: &[u8],
    ) -> Result<JsonValue, Error> {
        self.decode_common(
            event_name.as_bytes(),
            rkyv_bytes,
            |inner, args| {
                inner.decode_event_fn_export.call(&mut inner.store, args)
            },
            "decode_event",
        )
    }

    /// Returns the contract's JSON schema.
    ///
    /// The schema describes the structure of all inputs, outputs, and events
    /// supported by the contract driver.
    ///
    /// # Returns
    ///
    /// - `Ok(JsonValue)`: The contract schema as JSON
    /// - `Err(Error)`: If retrieval fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Schema retrieval failed
    /// - `Error::Json`: JSON parsing failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// let schema = driver.get_schema()?;
    /// println!("Schema: {}", schema);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_schema(&self) -> Result<JsonValue, Error> {
        let bytes = self.call_simple_output(
            |inner, args| inner.get_schema_fn.call(&mut inner.store, args),
            "get_schema",
        )?;
        Self::bytes_to_json(bytes)
    }

    /// Returns the contract driver's version string.
    ///
    /// This is typically a semantic version like "0.1.0".
    ///
    /// # Returns
    ///
    /// - `Ok(String)`: The version string
    /// - `Err(Error)`: If retrieval fails
    ///
    /// # Errors
    ///
    /// - `Error::WasmMemory`: Memory allocation failure
    /// - `Error::FfiError`: Version retrieval failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dusk_data_driver::reader::DriverReader;
    /// # use dusk_data_driver::Error;
    /// # fn main() -> Result<(), Error> {
    /// # let wasm_bytes = std::fs::read("driver.wasm").map_err(|e| Error::Other(e.to_string()))?;
    /// # let driver = DriverReader::new(&wasm_bytes)?;
    /// let version = driver.get_version()?;
    /// println!("Driver version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_version(&self) -> Result<String, Error> {
        let bytes = self.call_simple_output(
            |inner, args| inner.get_version_fn.call(&mut inner.store, args),
            "get_version",
        )?;
        String::from_utf8(bytes)
            .map_err(|e| Error::Other(format!("Invalid UTF-8: {e}")))
    }

    /// Common decode logic for `decode_input_fn`, `decode_output_fn`,
    /// `decode_event`
    fn decode_common<F>(
        &self,
        name_bytes: &[u8],
        rkyv_bytes: &[u8],
        call: F,
        op_name: &str,
    ) -> Result<JsonValue, Error>
    where
        F: FnOnce(
            &mut ReaderInner,
            CodecInputTuple,
        ) -> Result<i32, dusk_wasmtime::Error>,
    {
        let bytes = self
            .call_with_name_and_data(name_bytes, rkyv_bytes, call, op_name)?;
        Self::bytes_to_json(bytes)
    }

    /// Generic helper for calls with (name, data) -> output pattern
    fn call_with_name_and_data<F>(
        &self,
        name_bytes: &[u8],
        data_bytes: &[u8],
        call: F,
        op_name: &str,
    ) -> Result<Vec<u8>, Error>
    where
        F: FnOnce(
            &mut ReaderInner,
            CodecInputTuple,
        ) -> Result<i32, dusk_wasmtime::Error>,
    {
        let mut inner = self.inner.write();

        let (name_ptr, name_len) =
            Self::alloc_and_write(&mut inner, name_bytes)?;
        let (data_ptr, data_len) =
            Self::alloc_and_write(&mut inner, data_bytes)?;

        let result = Self::run_with_output_buffer(
            &mut inner,
            |inner, out_ptr, out_size| {
                call(
                    inner,
                    (name_ptr, name_len, data_ptr, data_len, out_ptr, out_size),
                )
                .map_err(|e| {
                    Error::FfiError(format!("{op_name} call failed: {e}"))
                })
            },
        );

        let _ = Self::dealloc(&mut inner, name_ptr, name_len);
        let _ = Self::dealloc(&mut inner, data_ptr, data_len);

        result
    }

    /// Helper for simple (`out_ptr`, `out_size`) -> output calls
    fn call_simple_output<F>(
        &self,
        call: F,
        op_name: &str,
    ) -> Result<Vec<u8>, Error>
    where
        F: FnOnce(
            &mut ReaderInner,
            (i32, i32),
        ) -> Result<i32, dusk_wasmtime::Error>,
    {
        let mut inner = self.inner.write();
        Self::run_with_output_buffer(&mut inner, |inner, out_ptr, out_size| {
            call(inner, (out_ptr, out_size)).map_err(|e| {
                Error::FfiError(format!("{op_name} call failed: {e}"))
            })
        })
    }

    /// Unified alloc+write for both strings and bytes
    fn alloc_and_write(
        inner: &mut ReaderInner,
        bytes: &[u8],
    ) -> Result<(i32, i32), Error> {
        let len = i32::try_from(bytes.len())
            .map_err(|_| Error::WasmMemory("Buffer too large".into()))?;

        let ptr = inner
            .alloc_fn
            .call(&mut inner.store, len)
            .map_err(|e| Error::WasmMemory(format!("alloc failed: {e}")))?;

        if ptr <= 0 {
            return Err(Error::WasmMemory(
                "alloc returned null or negative".into(),
            ));
        }

        // SAFETY: ptr and len are validated to be positive above
        #[allow(clippy::cast_sign_loss)]
        let start = ptr as usize;
        #[allow(clippy::cast_sign_loss)]
        let end = start + len as usize;

        inner
            .memory
            .data_mut(&mut inner.store)
            .get_mut(start..end)
            .ok_or_else(|| {
                Error::WasmMemory("Memory access out of bounds".into())
            })?
            .copy_from_slice(bytes);

        Ok((ptr, len))
    }

    /// Internal deallocation
    fn dealloc(
        inner: &mut ReaderInner,
        ptr: i32,
        len: i32,
    ) -> Result<(), Error> {
        inner
            .dealloc_fn
            .call(&mut inner.store, (ptr, len))
            .map_err(|e| Error::WasmMemory(format!("dealloc failed: {e}")))
    }

    /// Reads `[u32_le length][payload...]` from WASM memory.
    fn read_buffer(
        inner: &ReaderInner,
        ptr: i32,
        buf_size: i32,
    ) -> Result<Vec<u8>, Error> {
        if ptr < 0 || buf_size < 4 {
            return Err(Error::WasmMemory("Invalid buffer parameters".into()));
        }

        // SAFETY: ptr and buf_size are validated to be non-negative above
        #[allow(clippy::cast_sign_loss)]
        let ptr_usize = ptr as usize;
        #[allow(clippy::cast_sign_loss)]
        let buf_size_usize = buf_size as usize;

        let data = inner.memory.data(&inner.store);

        // Read 4-byte length prefix (little-endian)
        let len_bytes =
            data.get(ptr_usize..ptr_usize + 4).ok_or_else(|| {
                Error::WasmMemory("Cannot read length prefix".into())
            })?;

        let actual_size = u32::from_le_bytes([
            len_bytes[0],
            len_bytes[1],
            len_bytes[2],
            len_bytes[3],
        ]) as usize;

        // Check for overflow: actual_size + 4 must fit in buf_size
        if actual_size.saturating_add(4) > buf_size_usize {
            return Err(Error::WasmMemory(format!(
                "Buffer overflow: actual_size={actual_size}, buf_size={buf_size}"
            )));
        }

        let payload_start = ptr_usize + 4;
        let payload_end = payload_start + actual_size;
        let payload = data
            .get(payload_start..payload_end)
            .ok_or_else(|| Error::WasmMemory("Cannot read payload".into()))?;

        Ok(payload.to_vec())
    }

    /// Retrieves the last error message from WASM.
    fn get_last_error(inner: &mut ReaderInner) -> String {
        let out_ptr =
            match inner.alloc_fn.call(&mut inner.store, ERROR_BUFFER_SIZE) {
                Ok(ptr) if ptr != 0 => ptr,
                _ => return String::new(),
            };

        let _ = inner
            .get_last_error_fn
            .call(&mut inner.store, (out_ptr, ERROR_BUFFER_SIZE));

        let result = Self::read_buffer(inner, out_ptr, ERROR_BUFFER_SIZE)
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|e| Error::Other(format!("UTF-8 error: {e}")))
            })
            .unwrap_or_default();

        let _ = Self::dealloc(inner, out_ptr, ERROR_BUFFER_SIZE);
        result
    }

    /// Generic helper to allocate output buffer, run FFI call, handle errors.
    fn run_with_output_buffer<F>(
        inner: &mut ReaderInner,
        f: F,
    ) -> Result<Vec<u8>, Error>
    where
        F: FnOnce(&mut ReaderInner, i32, i32) -> Result<i32, Error>,
    {
        let out_ptr = inner
            .alloc_fn
            .call(&mut inner.store, OUTPUT_BUFFER_SIZE)
            .map_err(|e| Error::WasmMemory(format!("alloc failed: {e}")))?;

        if out_ptr == 0 {
            return Err(Error::WasmMemory("alloc returned null".into()));
        }

        let result = (|| {
            let code = f(inner, out_ptr, OUTPUT_BUFFER_SIZE)?;
            if code != 0 {
                let err_msg = Self::get_last_error(inner);
                return Err(Error::FfiError(format!(
                    "FFI returned {code}: {err_msg}"
                )));
            }
            Self::read_buffer(inner, out_ptr, OUTPUT_BUFFER_SIZE)
        })();

        let _ = Self::dealloc(inner, out_ptr, OUTPUT_BUFFER_SIZE);
        result
    }

    /// Convert bytes to `JsonValue`
    fn bytes_to_json(bytes: Vec<u8>) -> Result<JsonValue, Error> {
        let s = String::from_utf8(bytes)
            .map_err(|e| Error::Other(format!("Invalid UTF-8: {e}")))?;
        serde_json::from_str(&s).map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(OUTPUT_BUFFER_SIZE, 64 * 1024);
        assert_eq!(ERROR_BUFFER_SIZE, 1024);
    }
}
