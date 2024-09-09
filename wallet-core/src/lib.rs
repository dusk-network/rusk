// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![feature(try_trait_v2)]

#[cfg(target_family = "wasm")]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

extern crate alloc;

#[cfg(target_family = "wasm")]
#[macro_use]
mod ffi;

pub mod input;
pub mod keys;
pub mod notes;
pub mod transaction;

/// The seed used to generate the entropy for the keys
pub type Seed = [u8; 64];

pub mod prelude {
    //! Re-export of the most commonly used types and traits.
    pub use crate::input::MAX_INPUT_NOTES;
    pub use crate::keys;
}

use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Serializable, Write};

use execution_core::transfer::phoenix::{Note, ViewKey as PhoenixViewKey};

pub use notes::map_owned;

pub use notes::{map_owned, phoenix_balance, BalanceInfo};
