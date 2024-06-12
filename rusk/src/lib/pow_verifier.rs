// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Result;
use blake2::{Blake2s256, Digest};
use dusk_wallet_core::Transaction;

pub const POW_DIFFICULTY: usize = 16;

#[derive(Debug)]
pub struct PoW;

impl PoW {
    /// Produces a nonce such that hash of a given collection of bytes
    /// concatenated with the nonce has difficulty `difficulty`
    /// (meaning, its last `difficulty` bits will be zero)
    pub fn generate(bytes: impl AsRef<[u8]>, difficulty: usize) -> Vec<u8> {
        let mut nonce: u64 = 0;
        let mut hasher = Blake2s256::new();
        loop {
            hasher.update(bytes.as_ref());
            let nonce_bytes = nonce.to_le_bytes().to_vec();
            hasher.update(nonce_bytes.clone());
            if Self::verify_difficulty(
                &mut hasher.finalize_reset().iter().rev(),
                difficulty,
            ) {
                return nonce_bytes;
            }
            nonce += 1;
        }
    }

    /// Verifies the difficulty of a binary value according to the specified
    /// criteria.
    fn verify_difficulty<'a, I>(bytes: &mut I, difficulty: usize) -> bool
    where
        I: Iterator<Item = &'a u8>,
    {
        bytes.next().map_or(false, |b| {
            if difficulty <= 8 {
                b.trailing_zeros() as usize >= difficulty
            } else if b != &0 {
                false
            } else {
                Self::verify_difficulty(bytes, difficulty - 8)
            }
        })
    }

    pub fn verify(
        bytes: impl AsRef<[u8]>,
        nonce: impl AsRef<[u8]>,
        difficulty: usize,
    ) -> bool {
        let mut hasher = Blake2s256::new();
        hasher.update(bytes);
        hasher.update(nonce);
        Self::verify_difficulty(&mut hasher.finalize().iter().rev(), difficulty)
    }
}

pub fn verify_pow(tx: &Transaction, difficulty: usize) -> Result<bool> {
    let bytes = tx.to_hash_input_bytes();
    Ok(PoW::verify(bytes, tx.proof.as_slice(), difficulty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow() {
        let bytes = [3u8; 32].as_slice();
        let nonce = PoW::generate(bytes, POW_DIFFICULTY);
        assert!(PoW::verify(bytes, nonce.clone(), POW_DIFFICULTY));
    }
}
