// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake3::{Hasher, OUT_LEN};
use dusk_bytes::Serializable;
use dusk_core::abi::Event;

const BLOOM_BYTE_LEN: usize = 256;

/// A 2048 bit bloom filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bloom([u8; BLOOM_BYTE_LEN]);

impl Bloom {
    /// Create a new, empty, bloom.
    pub const fn new() -> Self {
        Self([0; BLOOM_BYTE_LEN])
    }

    /// Instantiate a new handle for IUF-style adding members, and membership
    /// checks.
    pub fn iuf(&mut self) -> BloomIuf {
        BloomIuf {
            hasher: Hasher::new(),
            bloom: self,
        }
    }

    /// Add a member to the bloom.
    #[allow(unused)]
    pub fn add(&mut self, bytes: impl AsRef<[u8]>) {
        let mut iuf = self.iuf();
        iuf.update(bytes);
        iuf.add();
    }

    /// Add an event to the bloom.
    #[allow(unused)]
    pub fn add_event(&mut self, event: &Event) {
        // We add the tuple (contract, topic) to allow for checking if an
        // event with the given topic was emitted in a given block.
        let mut iuf = self.iuf();
        iuf.update(event.source);
        iuf.update(&event.topic);
        iuf.add();

        // We also add the triple (contract, topic, data) to allow for checking
        // if the full event was emitted in the block.
        let mut iuf = self.iuf();
        iuf.update(event.source);
        iuf.update(&event.topic);
        iuf.update(&event.data);
        iuf.add();
    }

    /// Add some events to the bloom.
    #[allow(unused)]
    pub fn add_events<'a, I: IntoIterator<Item = &'a Event>>(
        &mut self,
        events: I,
    ) {
        for event in events {
            self.add_event(event);
        }
    }

    /// Returns true if the bytes are contained in the bloom.
    #[allow(unused)]
    pub fn contains(&mut self, bytes: impl AsRef<[u8]>) -> bool {
        let mut iuf = self.iuf();
        iuf.update(bytes);
        iuf.contains()
    }

    fn _add(&mut self, hash: &[u8; OUT_LEN]) {
        let (i0, v0, i1, v1, i2, v2) = bloom_values(hash);
        self.0[i0] |= v0;
        self.0[i1] |= v1;
        self.0[i2] |= v2;
    }

    fn _contains(&self, hash: &[u8; OUT_LEN]) -> bool {
        let (i0, v0, i1, v1, i2, v2) = bloom_values(hash);
        v0 == v0 & self.0[i0] && v1 == v1 & self.0[i1] && v2 == v2 & self.0[i2]
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Bloom> for [u8; BLOOM_BYTE_LEN] {
    fn from(bloom: Bloom) -> Self {
        bloom.0
    }
}

impl Serializable<BLOOM_BYTE_LEN> for Bloom {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; BLOOM_BYTE_LEN]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self(*buf))
    }

    fn to_bytes(&self) -> [u8; BLOOM_BYTE_LEN] {
        self.0
    }
}

fn bloom_values(hash: &[u8; OUT_LEN]) -> (usize, u8, usize, u8, usize, u8) {
    let v0 = 1 << (hash[1] & 0x7);
    let v1 = 1 << (hash[3] & 0x7);
    let v2 = 1 << (hash[5] & 0x7);

    let i0 =
        BLOOM_BYTE_LEN - ((be16(hash[0], hash[1]) & 0x7ff) >> 3) as usize - 1;
    let i1 =
        BLOOM_BYTE_LEN - ((be16(hash[2], hash[3]) & 0x7ff) >> 3) as usize - 1;
    let i2 =
        BLOOM_BYTE_LEN - ((be16(hash[4], hash[5]) & 0x7ff) >> 3) as usize - 1;

    (i0, v0, i1, v1, i2, v2)
}

fn be16(b0: u8, b1: u8) -> u16 {
    (b1 as u16) | ((b0 as u16) << 8)
}

/// Allows for an Init/Update/Finalize API for adding to and checking the
/// contents of a [`Bloom`] filter.
pub struct BloomIuf<'a> {
    hasher: Hasher,
    bloom: &'a mut Bloom,
}

impl BloomIuf<'_> {
    /// Updates the underlying hasher with the given `bytes`.
    pub fn update(&mut self, bytes: impl AsRef<[u8]>) {
        self.hasher.update(bytes.as_ref());
    }

    /// Finalize hashing and return true if the result is contained in the
    /// [`Bloom`],
    #[allow(unused)]
    pub fn contains(self) -> bool {
        let hash = self.hasher.finalize().into();
        self.bloom._contains(&hash)
    }

    /// Finalize hashing and add the result to the underlying [`Bloom`],
    /// returning the added bytes.
    pub fn add(self) -> [u8; OUT_LEN] {
        let hash = self.hasher.finalize().into();
        self.bloom._add(&hash);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership() {
        const INSERT_BYTES: &[u8; 43] =
            b"The quick brown fox jumps over the lazy dog";
        const NON_INSERT_BYTES: &[u8; 37] =
            b"Two driven jocks help fax my big quiz";

        let mut bloom = Bloom::new();

        assert!(
            !bloom.contains(INSERT_BYTES),
            "bloom should not contain the bytes before adding"
        );
        assert!(
            !bloom.contains(NON_INSERT_BYTES),
            "bloom should not contain the bytes that are never added"
        );

        bloom.add(INSERT_BYTES);

        assert!(
            bloom.contains(INSERT_BYTES),
            "bloom should contain the bytes after adding"
        );
        assert!(
            !bloom.contains(NON_INSERT_BYTES),
            "bloom should not contain the bytes that are never added"
        );
    }
}
