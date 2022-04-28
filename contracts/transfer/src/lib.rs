// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

mod error;
mod transfer;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use error::Error;
pub use transfer::{Call, Leaf, TransferContract};
pub type Map<K, V> = dusk_hamt::Hamt<K, V, ()>;

#[cfg(target_arch = "wasm32")]
pub(crate) use transfer::PublicKeyBytes;
