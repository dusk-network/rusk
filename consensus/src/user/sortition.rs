// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use sha3::{Digest, Sha3_256};

pub fn create_sortition_hash(seed: [u8; 32], round: u64, step: u8, i: i32) -> [u8; 32] {
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(round.to_le_bytes());
    hasher.update(i.to_le_bytes());
    hasher.update(step.to_le_bytes());
    hasher.update(seed.as_ref());

    // read hash digest
    let reader = hasher.finalize();
    reader.as_slice().try_into().expect("Wrong length")
}

#[cfg(test)]
mod tests {
    use crate::user::sortition::create_sortition_hash;
    use hex_literal::hex;

    #[test]
    pub fn test_sortition_hash() {
        assert_eq!(
            create_sortition_hash([3; 32], 10, 3, 1)[..],
            hex!("670eea4ae10ef4cdbdb3a7b56e9b06a4aafdffaa2562923791ceaffda486d5c7")[..]
        );
    }
}
