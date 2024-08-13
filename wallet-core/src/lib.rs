// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for interacting with Dusk's transfer and stake contracts.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]

pub mod keys;

/// Length of the seed of the generated rng.
pub const RNG_SEED: usize = 64;
