// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::alloc::{alloc, dealloc, Layout};
use core::slice;

use bytecheck::CheckBytes;
use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{check_archived_root, Archive, Deserialize};

use crate::ffi::error::ErrorCode;

#[cfg(target_family = "wasm")]
/// The alignment of the memory allocated by the FFI for WASM.
///
/// This is 1 because we're not allocating any complex data structures, and
/// just interacting with the memory directly.
const ALIGNMENT: usize = 1;

#[cfg(target_family = "wasm")]
/// Allocates a buffer of `len` bytes on the WASM memory.
#[no_mangle]
pub fn malloc(len: u32) -> u32 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        let ptr = alloc(layout);
        ptr as _
    }
}

#[cfg(target_family = "wasm")]
/// Frees a previously allocated buffer on the WASM memory.
#[no_mangle]
pub fn free(ptr: u32, len: u32) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        dealloc(ptr as _, layout);
    }
}

#[cfg(not(target_family = "wasm"))]
// Standard system alignment for non-WASM environment.
const ALIGNMENT: usize = std::mem::size_of::<usize>();

#[cfg(not(target_family = "wasm"))]
/// Allocates a buffer of `len` bytes on the system memory.
pub extern "C" fn allocate(len: usize) -> *mut u8 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        let ptr = alloc(layout);
        ptr
    }
}

#[cfg(not(target_family = "wasm"))]
#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, len: u32) {
    if ptr.is_null() {
        return;
    }

    let size = len as usize;

    let layout = match Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return,
    };

    unsafe {
        dealloc(ptr, layout);
    }
}

/// Read a buffer from the given pointer.
pub unsafe fn read_buffer<'a>(ptr: *const u8) -> &'a [u8] {
    let len = slice::from_raw_parts(ptr, 4);
    let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
    slice::from_raw_parts(ptr.add(4), len)
}

/// Parse the buffer
pub unsafe fn parse_buffer<T>(bytes: &[u8]) -> Result<T, ErrorCode>
where
    T: Archive,
    for<'a> T::Archived:
        CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>,
{
    let aligned = bytes.to_vec();
    let aligned_slice: &[u8] = &aligned;

    let result = check_archived_root::<T>(aligned_slice)
        .or(Err(ErrorCode::UnarchivingError))?
        .deserialize(&mut SharedDeserializeMap::default())
        .or(Err(ErrorCode::UnarchivingError));

    result
}

/// Checks and deserializes a value from the given po
pub unsafe fn from_buffer<T>(ptr: *const u8) -> Result<T, ErrorCode>
where
    T: Archive,
    for<'a> T::Archived:
        CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>,
{
    let bytes = read_buffer(ptr);

    parse_buffer::<T>(bytes)
}
