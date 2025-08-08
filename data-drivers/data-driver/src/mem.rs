// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

/// Allocate memory inside WASM for JS to write into.
/// Returns a pointer to a buffer of size `size`.
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

/// Deallocate memory previously allocated with `alloc`.
///
/// # Safety
/// The pointer must have been returned by `alloc` with the same `size`.
/// The memory must not have been previously deallocated.
/// After calling this function, the pointer must no longer be used.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    drop(Vec::from_raw_parts(ptr, size, size));
}
