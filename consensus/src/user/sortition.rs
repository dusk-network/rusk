// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use num_bigint::BigInt;
use num_bigint::Sign::Plus;

use sha3::{Digest, Sha3_256};

use node_data::ledger::Seed;

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq)]
pub struct Config {
    pub seed: Seed,
    pub round: u64,
    pub step: u8,
    pub committee_size: usize,
}

impl Config {
    pub fn new(
        seed: Seed,
        round: u64,
        step: u8,
        committee_size: usize,
    ) -> Config {
        Self {
            seed,
            round,
            step,
            committee_size,
        }
    }
}

// The deterministic procedure requires the set of active stakes,
// ordered in ascending order from oldest to newest, the latest global seed,
// current consensus round and current consensus step.

pub fn create_sortition_hash(cfg: &Config, counter: u32) -> [u8; 32] {
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(&cfg.seed.inner()[..]);
    hasher.update(cfg.round.to_le_bytes());
    hasher.update(cfg.step.to_le_bytes());
    hasher.update(counter.to_le_bytes());

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
// where  is the amount staked and  is the BLS public key corresponding to the
// stake.

#[cfg(test)]
mod tests {
    use node_data::ledger::Seed;
    use num_bigint::BigInt;

    use crate::user::sortition::{
        create_sortition_hash, generate_sortition_score, Config,
    };

    #[test]
    pub fn test_sortition_hash() {        
        let hash = [
            134, 22, 162, 136, 186, 35, 16, 207, 237, 50, 11, 236, 74, 189, 37,
            137, 101, 205, 53, 161, 248, 199, 195, 228, 68, 68, 95, 223, 239, 199,
            1, 7,
        ];

        assert_eq!(
            create_sortition_hash(
                &Config::new(Seed::from([3; 48]), 10, 3, 0),
                1
            )[..],
            hash[..],
        );
    }

    #[test]
    pub fn test_generate_sortition_score() {
        let dataset =
            vec![([3; 48], 123342342, 66422677), ([4; 48], 44443333, 22757716)];

        for (seed, total_weight, expected_score) in dataset {
            let hash = create_sortition_hash(
                &Config::new(Seed::from(seed), 10, 3, 0),
                1,
            );

            let total_weight = BigInt::from(total_weight);
            let res = generate_sortition_score(hash, &total_weight);

            assert_eq!(res, BigInt::from(expected_score));
        }
    }
}
