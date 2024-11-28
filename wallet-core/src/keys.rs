// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utilities to derive keys from the seed.

use alloc::vec::Vec;
use core::ops::Range;

use execution_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use execution_core::transfer::phoenix::{
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    ViewKey as PhoenixViewKey,
};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha12Rng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::Seed;

/// Generates a [`BlsSecretKey`] from a seed and index.
///
/// The randomness is generated using [`rng_with_index`].
#[must_use]
pub fn derive_bls_sk(seed: &Seed, index: u8) -> BlsSecretKey {
    // note that if we change the string used for the rng, all previously
    // generated keys will become invalid
    // NOTE: When breaking the keys, we will want to change the string too
    BlsSecretKey::random(&mut rng_with_index(seed, index, b"SK"))
}

/// Generates a [`BlsPublicKey`] from a seed and index.
///
/// The randomness is generated using [`rng_with_index`].
#[must_use]
pub fn derive_bls_pk(seed: &Seed, index: u8) -> BlsPublicKey {
    let mut sk = derive_bls_sk(seed, index);
    let pk = BlsPublicKey::from(&sk);
    sk.zeroize();

    pk
}

/// Generates a [`PhoenixSecretKey`] from a seed and index.
///
/// The randomness is generated using [`rng_with_index`].
#[must_use]
pub fn derive_phoenix_sk(seed: &Seed, index: u8) -> PhoenixSecretKey {
    // note that if we change the string used for the rng, all previously
    // generated keys will become invalid
    // NOTE: When breaking the keys, we will want to change the string too
    PhoenixSecretKey::random(&mut rng_with_index(seed, index, b"SSK"))
}

/// Generates multiple [`PhoenixSecretKey`] from a seed and a range of indices.
///
/// The randomness is generated using [`rng_with_index`].
#[must_use]
pub fn derive_multiple_phoenix_sk(
    seed: &Seed,
    index_range: Range<u8>,
) -> Vec<PhoenixSecretKey> {
    index_range
        .map(|index| derive_phoenix_sk(seed, index))
        .collect()
}

/// Generates a [`PhoenixPublicKey`] from its seed and index.
///
/// First the [`PhoenixSecretKey`] is derived with [`derive_phoenix_sk`], then
/// the public key is generated from it and the secret key is erased from
/// memory.
#[must_use]
pub fn derive_phoenix_pk(seed: &Seed, index: u8) -> PhoenixPublicKey {
    let mut sk = derive_phoenix_sk(seed, index);
    let pk = PhoenixPublicKey::from(&sk);
    sk.zeroize();

    pk
}

/// Generates a [`PhoenixViewKey`] from its seed and index.
///
/// First the [`PhoenixSecretKey`] is derived with [`derive_phoenix_sk`], then
/// the view key is generated from it and the secret key is erased from memory.
#[must_use]
pub fn derive_phoenix_vk(seed: &Seed, index: u8) -> PhoenixViewKey {
    let mut sk = derive_phoenix_sk(seed, index);
    let vk = PhoenixViewKey::from(&sk);
    sk.zeroize();

    vk
}

/// Creates a secure RNG from a seed with embedded index and termination
/// constant.
///
/// First the `seed` and then the little-endian representation of the key's
/// `index` are passed through SHA-256. A constant is then mixed in and the
/// resulting hash is then used to seed a `ChaCha12` CSPRNG, which is
/// subsequently used to generate the key.
#[must_use]
pub fn rng_with_index(
    seed: &Seed,
    index: u8,
    termination: &[u8],
) -> ChaCha12Rng {
    // NOTE: to not break the test-keys, we cast to a u64 here. Once we are
    // ready to use the new keys, the index should not be cast to a u64
    // anymore.
    let index = u64::from(index);
    let mut hash = Sha256::new();

    hash.update(seed);
    hash.update(index.to_le_bytes());
    hash.update(termination);

    let hash = hash.finalize().into();
    ChaCha12Rng::from_seed(hash)
}
