// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use num_bigint::BigInt;
use num_bigint::Sign::Plus;

use sha3::{Digest, Sha3_256};

use node_data::{bls::PublicKeyBytes, ledger::Seed};

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq)]
pub struct Config {
    seed: Seed,
    round: u64,
    step: u16,
    committee_size: usize,
    exclusion: Option<PublicKeyBytes>,
}

impl Config {
    pub fn new(
        seed: Seed,
        round: u64,
        step: u16,
        committee_size: usize,
        exclusion: Option<PublicKeyBytes>,
    ) -> Config {
        Self {
            seed,
            round,
            step,
            committee_size,
            exclusion,
        }
    }

    pub fn committee_size(&self) -> usize {
        self.committee_size
    }

    pub fn step(&self) -> u16 {
        self.step
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn exclusion(&self) -> Option<&PublicKeyBytes> {
        self.exclusion.as_ref()
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
            247, 14, 92, 48, 116, 139, 3, 5, 171, 135, 3, 182, 119, 212, 157,
            225, 128, 0, 254, 222, 137, 136, 24, 77, 124, 168, 221, 84, 82,
            110, 159, 206,
        ];

        assert_eq!(
            create_sortition_hash(
                &Config::new(Seed::from([3; 48]), 10, 3, 0, None),
                1
            )[..],
            hash[..],
        );
    }

    #[test]
    pub fn test_generate_sortition_score() {
        let dataset =
            vec![([3; 48], 123342342, 6458782), ([4; 48], 44443333, 13070642)];

        for (seed, total_weight, expected_score) in dataset {
            let hash = create_sortition_hash(
                &Config::new(Seed::from(seed), 10, 3, 0, None),
                1,
            );

            let total_weight = BigInt::from(total_weight);
            let res = generate_sortition_score(hash, &total_weight);

            assert_eq!(res, BigInt::from(expected_score));
        }
    }
}
