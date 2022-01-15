// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use blake2b_simd::{Params, State};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;

/// Hashes scalars and arbitrary slices of bytes using Blake2b-256, returning
/// a valid [`BlsScalar`].
///
/// The hashing cannot be proved inside a circuit, if that is desired, use
/// `poseidon_hash` instead.
pub struct Hasher {
    state: State,
}

impl Default for Hasher {
    fn default() -> Self {
        Hasher {
            state: Params::new().hash_length(BlsScalar::SIZE).to_state(),
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

    /// Get the output of the hasher.
    pub fn output(self) -> [u8; BlsScalar::SIZE] {
        let hasher = self;
        let mut buf = [0u8; BlsScalar::SIZE];
        buf.copy_from_slice(hasher.state.finalize().as_ref());

        // This is a workaround for the fact that `Blake2b` does not support
        // bitlengths that are not a multiple of 8.
        // We're going to zero out the last nibble of the hash to ensure
        // that the result can fit in a `BlsScalar`.
        buf[BlsScalar::SIZE - 1] &= 0xf;

        buf
    }

    /// Retrieve result and consume hasher instance.
    pub fn finalize(self) -> BlsScalar {
        BlsScalar::from_bytes(&self.output()).expect(
            "Conversion to BlsScalar should never fail after truncation",
        )
    }

    /// Compute hash of arbitrary data into a valid [`BlsScalar`].
    pub fn digest(data: impl AsRef<[u8]>) -> BlsScalar {
        let mut hasher = Hasher::new();
        hasher.update(data.as_ref());
        hasher.finalize()
    }
}
