// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! ![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
//! [![Repository](https://img.shields.io/badge/github-code--hasher-blueviolet?logo=github)](https://github.com/dusk-network/code-hasher)
//! [![Documentation](https://img.shields.io/badge/docs-code--hasher-blue?logo=rust)](https://docs.rs/code-hasher/)
//! # code-hasher
//!
//! Tiny proc macro library designed to hash a code block generating a unique
//! identifier for it which will get written into a `const` inside of the code
//! block.
//!
//! ## Example
//! ```rust
//! #[code_hasher::hash(SOME_CONST_NAME, version = "0.1.0")]
//! pub mod testing_module {
//!
//!     pub fn this_does_something() -> [u8; 32] {
//!         SOME_CONST_NAME
//!     }
//! }
//! ```
//!
//! Here, `SOME_CONST_NAME` has assigned as value the resulting hash of:
//! - The code contained inside `testing_module`.
//! - The version passed by the user (is optional). Not adding it will basically
//!   not hash this attribute and **WILL NOT** use any default alternatives.
//!
//! ## Licensing
//! This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0).
//! Please see [LICENSE](https://github.com/dusk-network/rusk/tree/master/macros/code-hasher/LICENSE) for further info.

use blake3::Hasher;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn hash(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut hasher = Hasher::new();
    let input_string = format!("{:?}", input);

    // We need to `let` this otherways it gets freed while borrowed.
    let attrs_string = attr.to_string();
    let attrs_split: Vec<&str> = attrs_string.split(',').collect();

    // Add the code version (passed as attribute) to the hasher.
    hasher.update(attrs_split.get(1).unwrap_or(&"").as_bytes());
    // Add code-block to the hasher.
    hasher.update(input_string.as_bytes());

    let id = *hasher.finalize().as_bytes();
    let mut token_stream = input.to_string();
    token_stream.pop();
    token_stream.push_str(&format!(
        "    const {}: [u8; 32] = {:?};",
        attrs_split.get(0).expect("Missing const name"),
        id
    ));
    token_stream.push_str(" }");
    token_stream.parse().expect(
        "Error parsing the output of the code-hasher macro as TokenStream",
    )
}
