// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod public_input;
pub use public_input::*;

pub(crate) enum Query {}

impl Query {
    pub const HASH: &str = "hash";
    pub const POSEIDON_HASH: &str = "poseidon_hash";
    pub const VERIFY_PROOF: &str = "verify_proof";
    pub const VERIFY_SCHNORR: &str = "verify_schnorr";
    pub const VERIFY_BLS: &str = "verify_bls";
}

pub(crate) enum Metadata {}

impl Metadata {
    pub const BLOCK_HEIGHT: &str = "block_height";
}

cfg_if::cfg_if! {
    if #[cfg(feature = "host")] {
        mod host;
        pub use host::*;
    } else {
        mod hosted;
        pub use hosted::*;
    }
}
