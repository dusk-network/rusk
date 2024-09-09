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

pub mod keys;
pub mod notes;
pub mod transaction;

/// The seed used to generate the entropy for the keys
pub type Seed = [u8; 64];

pub mod prelude {
    //! Re-export of the most commonly used types and traits.
    pub use crate::keys;
    pub use crate::notes::MAX_INPUT_NOTES;
}

pub use notes::balance::{
    calculate as phoenix_balance, TotalAmount as BalanceInfo,
};
pub use notes::owned::map as map_owned;
pub use notes::pick::notes as pick_notes;
