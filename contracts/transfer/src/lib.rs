// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(
    target_arch = "wasm32",
    feature(core_intrinsics, lang_items, alloc_error_handler)
)]

extern crate alloc;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub mod ops;

mod transfer;
pub use transfer::{Call, PublicKeyBytes, Transfer, TransferExecute};

#[cfg(target_arch = "wasm32")]
pub(crate) use transfer::{InternalCall, InternalCallResult};

#[cfg(test)]
mod tests;
