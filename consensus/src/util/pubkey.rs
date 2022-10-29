// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_bytes::Serializable;
use hex::ToHex;
use rand::rngs::StdRng;
use rand_core::SeedableRng;
use std::cmp::Ordering;

pub const PUBLIC_BLS_SIZE: usize = PublicKey::SIZE;

/// ConsensusPublicKey is a thin wrapper of dusk_bls12_381_sign::PublicKey
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct ConsensusPublicKey {
    inner: PublicKey,
    data: [u8; PUBLIC_BLS_SIZE],
}

impl ConsensusPublicKey {
    pub fn new(inner: PublicKey) -> Self {
        let data = inner.to_bytes();
        Self { inner, data }
    }

    /// from_sk_seed_u64 generates a sk from the specified seed and returns the associated public key
    pub fn from_sk_seed_u64(state: u64) -> Self {
        let rng = &mut StdRng::seed_from_u64(state);
        let sk = SecretKey::random(rng);

        Self::new(PublicKey::from(&sk))
    }

    /// to_bytes returns a copy of pk.pk_t().to_bytes() initialized on PublicKey::new call.
    /// NB Frequent use of pk_t().to_bytes() creates a noticeable perf overhead.
    pub fn bytes(&self) -> &[u8; PUBLIC_BLS_SIZE] {
        &self.data
    }

    pub fn inner(&self) -> &PublicKey {
        &self.inner
    }

    pub fn encode_short_hex(&self) -> String {
        let mut hex = self.bytes().encode_hex::<String>();
        hex.truncate(16);
        hex
    }
}

impl Default for ConsensusPublicKey {
    fn default() -> Self {
        Self {
            inner: PublicKey::default(),
            data: [0; PUBLIC_BLS_SIZE],
        }
    }
}

impl PartialOrd<ConsensusPublicKey> for ConsensusPublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl Ord for ConsensusPublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

impl std::fmt::Debug for ConsensusPublicKey {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut hex = self.data.encode_hex::<String>();
        hex.truncate(16);

        let debug_trait_builder = &mut ::core::fmt::Formatter::debug_tuple(f, "PublicKey");
        let _ = ::core::fmt::DebugTuple::field(debug_trait_builder, &hex);
        ::core::fmt::DebugTuple::finish(debug_trait_builder)
    }
}
