// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use num_bigint::BigInt;
use num_bigint::Sign::Plus;

use sha3::{Digest, Sha3_256};

use crate::commons::Seed;

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq)]
pub struct Config {
    pub seed: Seed,
    pub round: u64,
    pub step: u8,
    pub max_committee_size: usize,
}

impl Config {
    pub fn new(
        seed: Seed,
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

pub fn create_sortition_hash(cfg: &Config, counter: u32) -> [u8; 32] {
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(cfg.round.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    hasher.update(cfg.step.to_le_bytes());
    hasher.update(&cfg.seed.inner()[..]);

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
    use num_bigint::BigInt;

    use crate::{
        commons::Seed,
        user::sortition::{
            create_sortition_hash, generate_sortition_score, Config,
        },
    };

    #[test]
    pub fn test_sortition_hash() {
        let hash = [
            56, 81, 125, 39, 109, 105, 243, 20, 138, 196, 236, 197, 7, 155, 41,
            26, 217, 150, 9, 226, 76, 174, 67, 1, 230, 187, 81, 107, 192, 5,
            13, 73,
        ];
        assert_eq!(
            create_sortition_hash(
                &Config::new(Seed::new([3; 48]), 10, 3, 0),
                1
            )[..],
            hash[..],
        );
    }

    #[test]
    pub fn test_generate_sortition_score() {
        let dataset =
            vec![([3; 48], 123342342, 78899961), ([4; 48], 44443333, 5505832)];

        for data in dataset {
            let hash = create_sortition_hash(
                &Config::new(Seed::new(data.0), 10, 3, 0),
                1,
            );

            let total_weight = BigInt::from(data.1);
            let res = generate_sortition_score(hash, &total_weight);

            assert_eq!(res, BigInt::from(data.2));
        }
    }
}
