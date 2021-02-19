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
#![warn(missing_docs)]

//! This module contains the logic for the staking contract, which is used to
//! maintain the provisioner committee.

extern crate alloc;

#[cfg(target_arch = "wasm32")]
mod wasm;

/// This module contains all opcodes for the staking contract.
pub mod ops;

mod stake;
