// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The wallet specification.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(target_family = "wasm")]
mod ffi;

mod imp;
mod tx;

use alloc::vec::Vec;
use dusk_jubjub::BlsScalar;
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::Note;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};

pub use imp::*;
pub use tx::{Transaction, UnprovenTransaction, UnprovenTransactionInput};

/// The depth of an branch.
pub const POSEIDON_DEPTH: usize = 17;

/// Stores the cryptographic material necessary to derive cryptographic keys.
pub trait Store {
    /// The error type returned from the store.
    type Error;

    /// Retrieves the seed used to derive keys.
    fn get_seed(&self) -> Result<[u8; 64], Self::Error>;

    /// Retrieves a derived key from the store.
    ///
    /// The provided implementation simply gets the seed and regenerates the key
    /// every time with [`generate_ssk`]. It may be reimplemented to
    /// provide a cache for keys, or implement a different key generation
    /// algorithm.
    fn retrieve_key(&self, index: u64) -> Result<SecretSpendKey, Self::Error> {
        let seed = self.get_seed()?;
        Ok(generate_ssk(&seed, index))
    }
}

/// Generates a secret key from its seed and index.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. The resulting hash is then used to seed
/// a `ChaCha12` CSPRNG, which is subsequently used to generate the key.
pub fn generate_ssk(seed: &[u8; 64], index: u64) -> SecretSpendKey {
    let mut hash = Sha256::new();

    hash.update(&seed);
    hash.update(&index.to_le_bytes());

    let hash = hash.finalize().into();
    let mut rng = ChaCha12Rng::from_seed(hash);

    SecretSpendKey::random(&mut rng)
}

/// Types that are clients of the node's API.
pub trait NodeClient {
    /// Error returned by the node client.
    type Error;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error>;

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error>;

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_DEPTH>, Self::Error>;

    /// Requests that a node prove the given transaction.
    fn request_proof(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Proof, Self::Error>;
}
