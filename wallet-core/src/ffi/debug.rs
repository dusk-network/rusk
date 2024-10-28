// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implements `dbg!` and `eprintln!` macros, similar to those inthe Rust
//! standard library, with adaptations for use in a WASM environment.
//!
//! The `dbg!` macro outputs the value of an expression along with file and line
//! number details, useful for debugging. The `eprintln!` macro sends error
//! messages to the host environment.
//!
//! Unlike their standard counterparts, these macros are designed to be no-ops
//! in release builds, where optimizations are applied. This means no code is
//! generated for them in release mode, which improves performance and avoids
//! generating unnecessary debug information, as the WASM host environment is
//! expected to handle errors by aborting on panic, rather than logging.

#[cfg(any(debug_assertions, feature = "debug"))]
#[allow(unused_macros)]
#[macro_use]
pub mod enabled {
    use alloc::vec::Vec;

    extern "C" {
        fn sig(msg: *const u8); // Host function expects a pointer to a C-style string (null-terminated)
    }

    // Converts a Rust string to a C-style string (null-terminated)
    fn cstr(s: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(s.len() + 1); // Allocate space for string + null terminator
        bytes.extend_from_slice(s.as_bytes()); // Copy the string bytes
        bytes.push(0); // Add the null terminator
        bytes
    }

    // Send a signal to the host environment
    pub(crate) fn signal(message: &str) {
        let c_string = cstr(message); // Convert to C-string
        unsafe {
            sig(c_string.as_ptr()); // Send the C-string to the host function
        }
    }

    macro_rules! eprintln {
        // Match the format string with arguments (like the standard `println!`)
        ($($arg:tt)*) => {{
            // Use `format!` to create the formatted string
            let formatted = alloc::format!($($arg)*);
            // Call the `signal` function with the resulting string
            $crate::ffi::debug::enabled::signal(&formatted);
        }};
    }

    macro_rules! dbg {
        () => {
            eprintln!("[{}:{}:{}]", file!(), line!(), column!())
        };
        ($val:expr $(,)?) => {
            // Use of `match` here is intentional because it affects the lifetimes
            // of temporaries - https://stackoverflow.com/a/48732525/1063961
            match $val {
                tmp => {
                    eprintln!("[{}:{}:{}] {} = {:#?}",
                        file!(), line!(), column!(), stringify!($val), &tmp);
                    tmp
                }
            }
        };
        ($($val:expr),+ $(,)?) => {
            ($(dbg!($val)),+,)
        };
    }
}

#[cfg(not(any(debug_assertions, feature = "debug")))]
#[allow(unused_macros)]
#[macro_use]
pub mod disabled {
    macro_rules! dbg {
        ($val:expr) => {
            $val
        };
        ($($arg:tt)*) => {};
    }
    macro_rules! eprintln {
        ($($arg:tt)*) => {};
    }
}
