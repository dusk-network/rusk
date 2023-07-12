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
//! Tiny proc macro library designed to hash an impl block and generate a unique
//! identifier for it which will get written into an associated `const` of the
//! same type the impl block targets.
//!
//! ## Example
//! ```rust
//! struct MyStruct;
//!
//! #[code_hasher::hash(name = "SOME_CONST_NAME", version = "0.1.0")]
//! impl MyStruct {
//!     pub fn this_does_something() -> [u8; 32] {
//!         Self::SOME_CONST_NAME
//!     }
//! }
//! ```
//!
//! Here, `SOME_CONST_NAME` has assigned as value the resulting hash of:
//! - The `impl MyStruct` code block.
//! - The version passed by the user. The only value that is mixed in the hash
//!   is the major version number - if larger than 0 - or the minor version
//!   number.
//!
//! ## Licensing
//! This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0).
//! Please see [LICENSE](https://github.com/dusk-network/rusk/tree/master/macros/code-hasher/LICENSE) for further info.

use blake3::Hasher;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;
use semver::Version;
use syn::{parse_macro_input, AttributeArgs, ItemImpl};

#[derive(Debug, FromMeta)]
struct MacroArgs {
    name: proc_macro2::Ident,
    version: Option<String>,
}

#[proc_macro_attribute]
pub fn hash(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_string = input.to_string();

    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemImpl);

    let args = match MacroArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    // If the version changed in a "major way" the hash should also change
    let mut version = String::from("undefined");
    if let Some(v) = args.version {
        let v = match Version::parse(&v) {
            Ok(v) => v,
            Err(e) => panic!("version should be valid semver: {e}"),
        };

        if v.major == 0 {
            version = format!("0.{}.0", v.minor);
        } else {
            version = format!("{}.0.0", v.major);
        }
    }

    let mut hasher = Hasher::new();

    hasher.update(input_string.as_bytes());
    hasher.update(version.as_bytes());

    let hash: [u8; 32] = hasher.finalize().into();

    let const_name = args.name;
    let type_name = &input.self_ty;
    let generics = &input.generics;

    let output = quote! {
        impl #generics #type_name {
            const #const_name: [u8; 32] = [#(#hash),*];
        }

        #input
    };

    output.into()
}
