use std::alloc::{alloc, dealloc, Layout};
use std::{ptr, slice};

#[no_mangle]
pub extern "C" fn allocate(len: u32) -> *mut u8 {
    let size = len as usize;

    let layout = match Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    unsafe { alloc(layout) }
}

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

pub unsafe fn read_buffer<'a>(ptr: *const u8) -> &'a [u8] {
    let len = slice::from_raw_parts(ptr, 4);
    let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
    slice::from_raw_parts(ptr.add(4), len)
}
