// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::services::state_integrity::page_tree::{C_ARITY, P32_ARITY};
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Deserialize,
    Serialize,
    CheckBytes,
)]
#[archive(as = "Self")]
pub struct Hash([u8; blake3::OUT_LEN]);

impl Hash {
    pub fn new(bytes: &[u8]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(bytes);
        hasher.finalize()
    }

    pub fn as_bytes(&self) -> &[u8; blake3::OUT_LEN] {
        &self.0
    }
}

impl From<Hash> for [u8; blake3::OUT_LEN] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

impl From<[u8; blake3::OUT_LEN]> for Hash {
    fn from(bytes: [u8; blake3::OUT_LEN]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl dusk_merkle::Aggregate<C_ARITY> for Hash {
    const EMPTY_SUBTREE: Self = Hash([0; blake3::OUT_LEN]);

    fn aggregate(items: [&Self; C_ARITY]) -> Self {
        let mut hasher = Hasher::new();
        for item in items {
            hasher.update(item.as_bytes());
        }
        hasher.finalize()
    }
}

impl dusk_merkle::Aggregate<P32_ARITY> for Hash {
    const EMPTY_SUBTREE: Self = Hash([0; blake3::OUT_LEN]);

    fn aggregate(items: [&Self; P32_ARITY]) -> Self {
        let mut hasher = Hasher::new();
        for item in items {
            hasher.update(item.as_bytes());
        }
        hasher.finalize()
    }
}

#[derive(Debug, Clone)]
pub struct Hasher(blake3::Hasher);

impl Hasher {
    #[inline(always)]
    pub fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    #[inline(always)]
    pub fn update(&mut self, input: &[u8]) -> &mut Self {
        self.0.update(input);
        self
    }

    #[inline(always)]
    pub fn finalize(&self) -> Hash {
        let hash = self.0.finalize();
        Hash(hash.into())
    }
}
