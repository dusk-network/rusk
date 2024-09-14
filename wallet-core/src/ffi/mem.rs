// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::ffi::error::ErrorCode;

use core::slice;

use alloc::alloc::{alloc, dealloc, Layout};
use bytecheck::CheckBytes;
use rkyv::{
    check_archived_root, de::deserializers::SharedDeserializeMap,
    validation::validators::DefaultValidator, Archive, Deserialize,
};

/// The alignment of the memory allocated by the FFI.
///
/// This is 1 because we're not allocating any complex data structures, and
/// just interacting with the memory directly.
const ALIGNMENT: usize = 1;

/// Allocates a buffer of `len` bytes on the WASM memory.
#[no_mangle]
pub fn malloc(len: u32) -> u32 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        let ptr = alloc(layout);
        ptr as _
    }
}

/// Frees a previously allocated buffer on the WASM memory.
#[no_mangle]
pub fn free(ptr: u32, len: u32) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len as usize, ALIGNMENT);
        dealloc(ptr as _, layout);
    }
}

/// Checks and deserializes a value from the given po
pub unsafe fn from_buffer<T>(ptr: *const u8) -> Result<T, ErrorCode>
where
    T: Archive,
    for<'a> T::Archived:
        CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>,
{
    let len = slice::from_raw_parts(ptr, 4);
    let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
    let bytes = slice::from_raw_parts(ptr.add(4), len);

    let aligned = bytes.to_vec();
    let aligned_slice: &[u8] = &aligned;

    let result = check_archived_root::<T>(aligned_slice)
        .or(Err(ErrorCode::UnarchivingError))?
        .deserialize(&mut SharedDeserializeMap::default())
        .or(Err(ErrorCode::UnarchivingError));

    result
}
