// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::alloc::{alloc, dealloc, Layout};

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

// Currently we're not handling panic message in the WASM module; in the future
// we might want to enable it for `debug` releases.
mod panic_handling {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}
