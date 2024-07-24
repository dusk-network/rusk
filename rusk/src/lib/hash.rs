// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake2b_simd::{Params, State};

/// Hashes scalars and arbitrary slices of bytes using Blake2b, returning an
/// array of 32 bytes.
///
/// This hash cannot be proven inside a circuit, if that is desired, use
/// `poseidon_hash` instead.
pub struct Hasher {
    state: State,
}

impl Default for Hasher {
    fn default() -> Self {
        Hasher {
            state: Params::new().hash_length(64).to_state(),
        }
    }
}

impl Hasher {
    /// Create new hasher instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process data, updating the internal state.
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        self.state.update(data.as_ref());
    }

    /// Retrieve result and consume hasher instance.
    pub fn finalize(self) -> [u8; 32] {
        let hash = self.state.finalize();
        let mut a = [0u8; 32];
        a.clone_from_slice(&hash.as_array()[..32]);
        a
    }
}
