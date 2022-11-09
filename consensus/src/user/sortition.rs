// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use num_bigint::BigInt;
use num_bigint::Sign::Plus;

use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq)]
pub struct Config {
    pub seed: [u8; 32],
    pub round: u64,
    pub step: u8,
    pub max_committee_size: usize,
}

impl Config {
    pub fn new(
        seed: [u8; 32],
        round: u64,
        step: u8,
        max_committee_size: usize,
    ) -> Config {
        Self {
            seed,
            round,
            step,
            max_committee_size,
        }
    }
}

// The deterministic procedure requires the set of active stakes,
// ordered in ascending order from oldest to newest, the latest global seed,
// current consensus round and current consensus step.

pub fn create_sortition_hash(cfg: &Config, i: i32) -> [u8; 32] {
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(cfg.round.to_le_bytes());
    hasher.update(i.to_le_bytes());
    hasher.update(cfg.step.to_le_bytes());
    hasher.update(cfg.seed);

    // read hash digest
    let reader = hasher.finalize();
    reader.as_slice().try_into().expect("Wrong length")
}

// Generate a score from the given hash and total stake weight
pub fn generate_sortition_score(
    hash: [u8; 32],
    total_weight: &BigInt,
) -> BigInt {
    let num = BigInt::from_bytes_be(Plus, hash.as_slice());
    num % total_weight
}

// The set of active stakes consists of tuples ,
// where  is the amount staked and  is the BLS public key corresponding to the stake.

#[cfg(test)]
mod tests {
    use crate::user::sortition::{
        create_sortition_hash, generate_sortition_score, Config,
    };
    use hex_literal::hex;
    use num_bigint::BigInt;

    #[test]
    pub fn test_sortition_hash() {
        assert_eq!(
            create_sortition_hash(&Config::new([3; 32], 10, 3, 0), 1)[..],
            hex!("670eea4ae10ef4cdbdb3a7b56e9b06a4aafdffaa2562923791ceaffda486d5c7")[..]
        );
    }

    #[test]
    pub fn test_generate_sortition_score() {
        let dataset = vec![
            (
                hex!("670eea4ae10ef4cdbdb3a7b56e9b06a4aafdffaa2562923791ceaffda486d5c7"),
                123342342,
                30711969,
            ),
            (
                hex!("2e99758548972a8e8822ad47fa1017ff72f06f3ff6a016851f45c398732bc50c"),
                44443333,
                11567776,
            ),
        ];

        for data in dataset {
            let hash = create_sortition_hash(&Config::new(data.0, 10, 3, 0), 1);

            let total_weight = BigInt::from(data.1);
            let res = generate_sortition_score(hash, &total_weight);

            assert_eq!(res, BigInt::from(data.2));
        }
    }
}
