// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake2b_simd::{Params, State};
use execution_core::BlsScalar;

/// Hashes scalars and arbitrary slices of bytes using Blake2b, returning a
/// valid [`BlsScalar`]. Using the `Hasher` yields the same result as when using
/// `BlsScalar::hash_to_scalar`.
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

    /// Process input data in a chained manner.
    pub fn chain_update(self, data: impl AsRef<[u8]>) -> Self {
        let mut hasher = self;
        hasher.state.update(data.as_ref());
        hasher
    }

    /// Retrieve result and consume hasher instance.
    pub fn finalize(self) -> BlsScalar {
        BlsScalar::from_bytes_wide(self.state.finalize().as_array())
    }

    /// Compute hash of arbitrary data into a valid [`BlsScalar`]. This
    /// equivalent to using `BlsScalar::hash_to_scalar`.
    pub fn digest(data: impl AsRef<[u8]>) -> BlsScalar {
        let mut hasher = Hasher::new();
        hasher.update(data.as_ref());
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::{OsRng, RngCore};

    #[test]
    fn test_hash() {
        let mut input = [0u8; 100000];
        OsRng.fill_bytes(&mut input[..]);

        let mut hasher = Hasher::new();
        for input_chunk in input.chunks(100) {
            hasher.update(input_chunk);
        }
        let hash = hasher.finalize();

        assert_eq!(hash, BlsScalar::hash_to_scalar(&input));
        assert_eq!(hash, Hasher::digest(&input));
    }
}
