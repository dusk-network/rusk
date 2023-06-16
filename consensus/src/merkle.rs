// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_merkle::{Aggregate, Tree};
use sha3::{Digest, Sha3_256};

pub const ARITY: usize = 2;

#[derive(Clone, Copy)]
pub struct Hash([u8; 32]);

impl<I: Into<[u8; 32]>> From<I> for Hash {
    fn from(bytes: I) -> Self {
        Self(bytes.into())
    }
}

pub const EMPTY_NODE: Hash = Hash([0; 32]);

impl Aggregate<ARITY> for Hash {
    const EMPTY_SUBTREE: Self = EMPTY_NODE;

    /// old golang implementation duplicates the missing leaf
    ///
    /// E.g: for (H=3, A=2)
    ///
    /// if you have
    /// 1 2 3 4 5
    /// it will create duplicates
    /// 1 2 3 4 5 5 5 5
    ///
    /// if you have
    /// 1 2 3 4 5 6
    /// it creates
    /// 1 2 3 4 5 6 5 6
    fn aggregate(items: [&Self; ARITY]) -> Self {
        let mut hasher = Sha3_256::new();
        let mut prev = &[0u8; 32];
        for item in items {
            if item.0 == EMPTY_NODE.0 {
                hasher.update(prev);
            } else {
                hasher.update(item.0);
                prev = &item.0;
            };
        }
        Self::from(hasher.finalize())
    }
}

struct BinaryMerkle<const H: usize> {
    tree: Tree<Hash, H, ARITY>,
}

impl<const H: usize> BinaryMerkle<H> {
    const fn new() -> Self {
        Self { tree: Tree::new() }
    }

    fn insert<N: Into<Hash>>(&mut self, val: N) {
        let val: [u8; 32] = Into::<Hash>::into(val).0;
        self.tree.insert(self.tree.len(), val);
    }

    fn root_from_values<N: Into<Hash> + Copy>(values: &[N]) -> [u8; 32] {
        let mut tree = Self::new();
        for &val in values {
            tree.insert(val.into())
        }
        let (shrunken_root, _) = tree.tree.smallest_subtree();
        shrunken_root.0
    }
}

/// Calculate the root of a dynamic merkle tree (containing up to 2^15 elements)
/// in the same way of how dusk-blockchain does
///
/// For reference impl check here https://github.com/dusk-network/dusk-crypto/blob/master/merkletree/merkletree.go
pub fn merkle_root<N: Into<Hash> + Copy>(values: &[N]) -> [u8; 32] {
    if values.is_empty() {
        return EMPTY_NODE.0;
    }
    BinaryMerkle::<15>::root_from_values(values)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Clone, Copy)]
    struct HashableStr(&'static str);

    impl Into<Hash> for HashableStr {
        fn into(self) -> Hash {
            let mut hasher = Sha3_256::new();
            hasher.update(self.0.as_bytes());
            hasher.finalize().into()
        }
    }

    #[test]
    fn golang_compatibility() {
        let mut test_vectors = vec![];

        // Test merkle tree againt same vectors used by
        // https://github.com/dusk-network/dusk-crypto/blob/more_fixtures/merkletree/merkletree_test.go
        test_vectors.push((vec![], EMPTY_NODE.0));

        test_vectors.push((
            vec![
                HashableStr("Hello"),
                HashableStr("Hi"),
                HashableStr("Hey"),
                HashableStr("Hola"),
            ],
            [
                32, 188, 172, 153, 245, 171, 51, 156, 161, 201, 80, 58, 155,
                97, 1, 79, 86, 175, 244, 91, 137, 105, 238, 155, 233, 126, 112,
                151, 195, 101, 37, 220,
            ],
        ));

        test_vectors.push((
            vec![HashableStr("Bella")],
            [
                21, 37, 141, 144, 92, 108, 151, 168, 167, 108, 172, 79, 204,
                99, 154, 196, 242, 247, 38, 145, 254, 228, 141, 104, 75, 210,
                148, 101, 74, 166, 1, 65,
            ],
        ));

        test_vectors.push((
            vec![
                HashableStr("Bella"),
                HashableStr("Ciao"),
                HashableStr("Stop"),
            ],
            [
                237, 79, 47, 121, 34, 152, 157, 123, 29, 245, 148, 185, 242,
                38, 186, 47, 27, 182, 232, 127, 84, 7, 77, 127, 146, 196, 83,
                8, 245, 190, 21, 5,
            ],
        ));

        test_vectors.push((
            vec![
                HashableStr("Bella"),
                HashableStr("Ciao"),
                HashableStr("Ndo"),
                HashableStr("Scappi"),
            ],
            [
                88, 206, 79, 67, 184, 243, 45, 145, 95, 208, 173, 187, 93, 119,
                173, 216, 206, 250, 233, 54, 65, 0, 166, 185, 102, 156, 49,
                227, 107, 3, 178, 119,
            ],
        ));

        test_vectors.push((
            vec![
                HashableStr("Bella"),
                HashableStr("Ciao"),
                HashableStr("Ndo"),
                HashableStr("Scappi"),
                HashableStr("Stop"),
            ],
            [
                254, 132, 247, 13, 76, 173, 142, 94, 29, 216, 48, 25, 120, 142,
                205, 65, 160, 88, 186, 233, 10, 143, 123, 79, 53, 22, 24, 55,
                47, 130, 228, 238,
            ],
        ));

        test_vectors.push((
            vec![
                HashableStr("Bella"),
                HashableStr("Ciao"),
                HashableStr("Ndo"),
                HashableStr("Scappi"),
                HashableStr("Stop"),
                HashableStr("Pari"),
            ],
            [
                232, 216, 240, 248, 181, 80, 60, 110, 63, 103, 197, 226, 130,
                128, 226, 245, 173, 218, 140, 195, 246, 109, 134, 119, 4, 192,
                166, 120, 163, 2, 95, 230,
            ],
        ));

        for (values, expected_hash) in test_vectors {
            let actual = merkle_root(&values[..]);
            assert_eq!(actual, expected_hash)
        }
    }
}
