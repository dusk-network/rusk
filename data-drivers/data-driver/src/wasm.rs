// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! FFI interface for contract driver WASM bindings.
//!
//! Provides C-compatible functions to interact with the contract driver.

use crate::ConvertibleContract;
use alloc::string::{String, ToString};
use alloc::{format, vec::Vec};
use core::cell::UnsafeCell;
use core::convert::TryFrom;
use core::ffi::c_char;
// use core::ffi::c_void;
use core::{ffi::CStr, ptr, slice};

/// FFI-compatible error codes returned by WASM functions.
#[repr(i32)]
pub enum ErrorCode {
    /// No error occurred.
    Ok = 0,
    /// Input was invalid.
    InvalidInput = 1,
    /// Operation failed during execution.
    OperationError = 2,
    /// Contract driver was not initialized.
    DriverNotInitialized = 3,
    /// Provided output buffer was too small.
    BufferTooSmall = 4,
}

/// Global contract driver instance.
static mut CONTRACT_DRIVER: Option<&'static dyn ConvertibleContract> = None;

/// Last error holder structure.
struct LastError {
    value: Option<String>,
}

/// Wrapper for `UnsafeCell` to safely hold a global error in single-threaded
/// WASM.
struct GlobalLastError(UnsafeCell<LastError>);
unsafe impl Sync for GlobalLastError {}

/// Global instance of the last error storage.
static LAST_ERROR: GlobalLastError =
    GlobalLastError(UnsafeCell::new(LastError { value: None }));

/// Set the last error message.
fn set_last_error(err: String) {
    unsafe {
        (*LAST_ERROR.0.get()).value = Some(err);
    }
}

/// Take and clear the last error message.
fn take_last_error() -> Option<String> {
    unsafe { (*LAST_ERROR.0.get()).value.take() }
}

/// Write a byte slice into a WASM buffer (with a 4-byte length prefix).
///
/// # Safety
/// Caller must ensure that `out_ptr` points to a valid buffer of at least
/// `out_buf_size` bytes.
unsafe fn write_to_wasm_buffer(
    data: &[u8],
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> Result<(), ErrorCode> {
    let len =
        u32::try_from(data.len()).map_err(|_| ErrorCode::BufferTooSmall)?;
    if data.len() + 4 > out_buf_size {
        return Err(ErrorCode::BufferTooSmall);
    }
    let len_bytes = len.to_le_bytes();
    ptr::copy_nonoverlapping(len_bytes.as_ptr(), out_ptr, 4);
    ptr::copy_nonoverlapping(data.as_ptr(), out_ptr.add(4), data.len());
    Ok(())
}

/// Utility wrapper to run a contract operation and write the result into a WASM
/// buffer.
///
/// # Safety
/// Caller must ensure that `out_ptr` points to a valid buffer of at least
/// `out_buf_size` bytes.
unsafe fn run_wasm_export<F>(
    out_ptr: *mut u8,
    out_buf_size: usize,
    f: F,
) -> ErrorCode
where
    F: FnOnce(&dyn ConvertibleContract) -> Result<Vec<u8>, String>,
{
    let Some(driver) = CONTRACT_DRIVER else {
        return ErrorCode::DriverNotInitialized;
    };

    match f(driver) {
        Ok(data) => match write_to_wasm_buffer(&data, out_ptr, out_buf_size) {
            Ok(()) => ErrorCode::Ok,
            Err(e) => {
                set_last_error("Output buffer too small".to_string());
                e
            }
        },
        Err(msg) => {
            set_last_error(msg);
            ErrorCode::OperationError
        }
    }
}

// /// Opaque handle for a `ConvertibleContract` implementor.
// ///
// /// Used for FFI to pass a trait object across the WASM/C boundary.
// #[repr(C)]
// pub struct ConvertibleContractHandle {
//     /// Pointer to the data part of the trait object.
//     pub data: *const c_void,
//     /// Pointer to the vtable part of the trait object.
//     pub vtable: *const c_void,
// }

// /// Initializes the global contract driver instance.
// ///
// /// # Safety
// /// Caller must ensure `handle` is a valid pointer pair to a
// /// `ConvertibleContract` implementor.
// #[no_mangle]
// pub unsafe extern "C" fn init_contract_driver(
//     handle: ConvertibleContractHandle,
// ) -> ErrorCode {
//     if handle.data.is_null() || handle.vtable.is_null() {
//         return ErrorCode::InvalidInput;
//     }
//     let driver: &dyn ConvertibleContract =
//         core::mem::transmute((handle.data, handle.vtable));
//     CONTRACT_DRIVER = Some(driver);
//     ErrorCode::Ok
// }

/// Retrieves and clears the last error message.
///
/// # Safety
/// Caller must ensure that `out_ptr` points to a valid buffer of at least
/// `out_buf_size` bytes.
#[no_mangle]
pub unsafe extern "C" fn get_last_error(
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    let err = take_last_error().unwrap_or_default();
    match write_to_wasm_buffer(err.as_bytes(), out_ptr, out_buf_size) {
        Ok(()) => ErrorCode::Ok,
        Err(e) => e,
    }
}

/// Encodes an input function call with JSON parameters.
///
/// # Safety
/// Caller must ensure that all pointers are valid and null-terminated.
#[no_mangle]
pub unsafe extern "C" fn encode_input_fn(
    fn_name: *const c_char,
    json: *const c_char,
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    run_wasm_export(out_ptr, out_buf_size, |driver| {
        let fn_name = CStr::from_ptr(fn_name)
            .to_str()
            .map_err(|e| format!("Invalid fn_name: {e}"))?;
        let json = CStr::from_ptr(json)
            .to_str()
            .map_err(|e| format!("Invalid json: {e}"))?;
        driver
            .encode_input_fn(fn_name, json)
            .map_err(|e| format!("{e:?}"))
    })
}

/// Decodes input function parameters from a serialized format.
///
/// # Safety
/// Caller must ensure that all pointers are valid and buffers are properly
/// sized.
#[no_mangle]
pub unsafe extern "C" fn decode_input_fn(
    fn_name: *const c_char,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    run_wasm_export(out_ptr, out_buf_size, |driver| {
        let fn_name = CStr::from_ptr(fn_name)
            .to_str()
            .map_err(|e| format!("Invalid fn_name: {e}"))?;
        let rkyv = slice::from_raw_parts(rkyv_ptr, rkyv_len);
        driver
            .decode_input_fn(fn_name, rkyv)
            .map(|v| v.to_string().into_bytes())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Decodes output function results from a serialized format.
///
/// # Safety
/// Caller must ensure that all pointers are valid and buffers are properly
/// sized.
#[no_mangle]
pub unsafe extern "C" fn decode_output_fn(
    fn_name: *const c_char,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    run_wasm_export(out_ptr, out_buf_size, |driver| {
        let fn_name = CStr::from_ptr(fn_name)
            .to_str()
            .map_err(|e| format!("Invalid fn_name: {e}"))?;
        let rkyv = slice::from_raw_parts(rkyv_ptr, rkyv_len);
        driver
            .decode_output_fn(fn_name, rkyv)
            .map(|v| v.to_string().into_bytes())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Decodes an event from a serialized format.
///
/// # Safety
/// Caller must ensure that all pointers are valid and buffers are properly
/// sized.
#[no_mangle]
pub unsafe extern "C" fn decode_event(
    event_name: *const c_char,
    rkyv_ptr: *const u8,
    rkyv_len: usize,
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    run_wasm_export(out_ptr, out_buf_size, |driver| {
        let event_name = CStr::from_ptr(event_name)
            .to_str()
            .map_err(|e| format!("Invalid event_name: {e}"))?;
        let rkyv = slice::from_raw_parts(rkyv_ptr, rkyv_len);
        driver
            .decode_event(event_name, rkyv)
            .map(|v| v.to_string().into_bytes())
            .map_err(|e| format!("{e:?}"))
    })
}

/// Retrieves the contract schema as a serialized string.
///
/// # Safety
/// Caller must ensure that `out_ptr` points to a valid buffer of at least
/// `out_buf_size` bytes.
#[no_mangle]
pub unsafe extern "C" fn get_schema(
    out_ptr: *mut u8,
    out_buf_size: usize,
) -> ErrorCode {
    run_wasm_export(out_ptr, out_buf_size, |driver| {
        Ok(driver.get_schema().into_bytes())
    })
}

/// Generates a WASM `init` entrypoint for a given contract driver type.
///
/// Usage:
/// ```ignore
/// generate_wasm_entrypoint!(MyDriver);
/// ```
///
/// This will create:
/// ```ignore
/// #[no_mangle]
/// pub unsafe extern "C" fn init() {
///     static INSTANCE: MyDriver = MyDriver;
///     dusk_data_driver::init_contract_driver(&INSTANCE);
/// }
/// ```
#[macro_export]
macro_rules! generate_wasm_entrypoint {
    ($driver_type:ty) => {
        static mut INSTANCE: Option<$driver_type> = None;
        #[no_mangle]
        #[doc = "Initializes and registers the contract driver.\n\n\
                 # Safety\n\
                 Must be called exactly once at module startup before using any FFI functions."]
        pub unsafe extern "C" fn init() {
            INSTANCE = Some(<$driver_type>::default());
            $crate::wasm::init_contract_driver(INSTANCE.as_ref().unwrap());
        }
    };
}

#[cfg(all(target_family = "wasm", feature = "wasm-export"))]
/// Safe Rust API for contract implementors.
///
/// # Safety
/// Must be called exactly once at startup.
pub unsafe fn init_contract_driver(
    driver: &'static dyn ConvertibleContract,
) -> ErrorCode {
    unsafe { CONTRACT_DRIVER = Some(driver) };
    ErrorCode::Ok
}
