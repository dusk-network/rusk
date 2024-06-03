// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use num_bigint::BigInt;
use num_bigint::Sign::Plus;

use sha3::{Digest, Sha3_256};

use node_data::{bls::PublicKeyBytes, ledger::Seed, StepName};

use crate::config::{
    PROPOSAL_COMMITTEE_SIZE, RATIFICATION_COMMITTEE_SIZE,
    VALIDATION_COMMITTEE_SIZE,
};

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
        iteration: u8,
        step: StepName,
        exclusion: Option<PublicKeyBytes>,
    ) -> Config {
        let committee_size = match step {
            StepName::Proposal => PROPOSAL_COMMITTEE_SIZE,
            StepName::Ratification => RATIFICATION_COMMITTEE_SIZE,
            StepName::Validation => VALIDATION_COMMITTEE_SIZE,
        };
        let step = step.to_step(iteration);
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
    hasher.update(cfg.step.to_le_bytes());
    hasher.update(counter.to_le_bytes());

    // read hash digest
    let reader = hasher.finalize();
    reader.as_slice().try_into().expect("Wrong length")
}

/// Generate a score from the given hash and total stake weight
pub fn generate_sortition_score(
    hash: [u8; 32],
    total_weight: &BigInt,
) -> BigInt {
    let num = BigInt::from_bytes_be(Plus, hash.as_slice());
    num % total_weight
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::user::committee::Committee;
    use crate::user::provisioners::{Provisioners, DUSK};
    use crate::user::sortition::Config;
    use dusk_bytes::DeserializableSlice;
    use execution_core::{StakePublicKey, StakeSecretKey};

    use node_data::ledger::Seed;

    impl Config {
        pub fn raw(
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
    }

    #[test]
    pub fn test_sortition_hash() {
        let hash = [
            102, 194, 89, 58, 228, 163, 25, 223, 222, 214, 246, 137, 213, 168,
            65, 13, 83, 130, 73, 187, 213, 145, 71, 20, 64, 167, 93, 251, 151,
            200, 167, 168,
        ];

        assert_eq!(
            create_sortition_hash(
                &Config::raw(Seed::from([3; 48]), 10, 3, 0, None),
                1
            )[..],
            hash[..],
        );
    }

    #[test]
    pub fn test_generate_sortition_score() {
        let dataset = vec![
            ([3; 48], 123342342, 40818748),
            ([4; 48], 44443333, 26458886),
        ];

        for (seed, total_weight, expected_score) in dataset {
            let hash = create_sortition_hash(
                &Config::raw(Seed::from(seed), 10, 3, 0, None),
                1,
            );

            let total_weight = BigInt::from(total_weight);
            let res = generate_sortition_score(hash, &total_weight);

            assert_eq!(res, BigInt::from(expected_score));
        }
    }

    #[test]
    fn test_deterministic_sortition_1() {
        let p = generate_provisioners(5);

        let committee_size = 64;

        // Execute sortition with specific config
        let cfg = Config::raw(Seed::default(), 1, 1, 64, None);

        let committee = Committee::new(&p, &cfg);

        // Verify expected committee size
        assert_eq!(
            committee_size,
            committee.get_occurrences().iter().sum::<usize>()
        );

        // Verify expected distribution
        assert_eq!(vec![8, 20, 14, 22], committee.get_occurrences());
    }

    #[test]
    fn test_deterministic_sortition_2() {
        let p = generate_provisioners(5);

        let committee_size = 45;
        let cfg =
            Config::raw(Seed::from([3u8; 48]), 7777, 8, committee_size, None);

        let committee = Committee::new(&p, &cfg);
        assert_eq!(
            committee_size,
            committee.get_occurrences().iter().sum::<usize>()
        );
        assert_eq!(vec![3, 17, 9, 16], committee.get_occurrences());
    }

    #[test]
    fn test_deterministic_sortition_2_exclusion() {
        let p = generate_provisioners(5);

        let seed = Seed::from([3u8; 48]);
        let round = 7777;
        let committee_size = 45;
        let iteration = 2;
        let relative_step = 2;
        let step = iteration as u16 * 3 + relative_step;

        let cfg = Config::raw(seed, round, step, committee_size, None);
        let generator = p.get_generator(iteration, seed, round);
        let committee = Committee::new(&p, &cfg);

        committee
            .iter()
            .find(|&p| p.bytes() == &generator)
            .expect("Generator to be included");
        assert_eq!(
            committee_size,
            committee.get_occurrences().iter().sum::<usize>()
        );
        assert_eq!(vec![3, 17, 9, 16], committee.get_occurrences());

        // Run the same extraction, with the generator excluded
        let cfg =
            Config::raw(seed, round, step, committee_size, Some(generator));
        let committee = Committee::new(&p, &cfg);

        assert!(
            committee
                .iter()
                .find(|&p| p.bytes() == &generator)
                .is_none(),
            "Generator to be excluded"
        );
        assert_eq!(
            committee_size,
            committee.get_occurrences().iter().sum::<usize>()
        );
        assert_eq!(vec![5, 23, 17], committee.get_occurrences());
    }

    #[test]
    fn test_quorum() {
        let p = generate_provisioners(5);

        let cfg = Config::raw(Seed::default(), 7777, 8, 64, None);

        let c = Committee::new(&p, &cfg);
        assert_eq!(c.super_majority_quorum(), 43);
    }

    #[test]
    fn test_intersect() {
        let p = generate_provisioners(10);

        let cfg = Config::raw(Seed::default(), 1, 3, 200, None);
        // println!("{:#?}", p);

        let c = Committee::new(&p, &cfg);
        // println!("{:#?}", c);

        let max_bitset = (2_i32.pow((c.size()) as u32) - 1) as u64;
        println!("max_bitset: {} / {:#064b} ", max_bitset, max_bitset);

        for bitset in 0..max_bitset {
            //println!("bitset: {:#064b}", bitset);
            let result = c.intersect(bitset);
            assert_eq!(
                c.bits(&result),
                bitset,
                "testing with  bitset:{}",
                bitset
            );
        }
    }

    fn generate_provisioners(n: usize) -> Provisioners {
        let sks = [
            "7f6f2ccdb23f2abb7b69278e947c01c6160a31cf02c19d06d0f6e5ab1d768b15",
            "611830d3641a68f94a690dcc25d1f4b0dac948325ac18f6dd32564371735f32c",
            "1fbec814b18b1d4c3eaa7cec41007e04bf0a98453b06ec7582aa29882c52eb3e",
            "ecd9c4a53ea15f18447b08fb96a13c5ab7dc7d24067b102fcbaaf7b39ca52e2d",
            "e463bcb1a6e57288ffd4671503082fa8656e3eacb78fb1925f8a7c76400e8e15",
            "7a19fb2d099a9557f7c10c2efbb8b101d9e0ec85610d5c74a887d1d4fb8d2827",
            "4dbad51eb408af559dd91bbbed8dbeae0a2c89e0e05f0cce87c98652a8437f1f",
            "befba86ae9e0c207865f7e24e8349d4ecdbc8b0f4632842499a0dfa60568e20a",
            "b260b8a10343bf5a5dacb4f1d32d06c4fdddc9981a3619fbc0a5cd9eb30f3334",
            "87a9779748888da5d96bbbce041b5109c6ffc0c4f30561c0170384a5922d9e21",
        ];
        let sks: Vec<_> = sks
            .iter()
            .take(n)
            .map(|hex| hex::decode(hex).expect("valid hex"))
            .map(|data| {
                StakeSecretKey::from_slice(&data[..]).expect("valid secret key")
            })
            .collect();

        let mut p = Provisioners::empty();
        for (i, sk) in sks.iter().enumerate().skip(1) {
            let stake_value = 1000 * (i) as u64 * DUSK;
            let stake_pk =
                node_data::bls::PublicKey::new(StakePublicKey::from(sk));
            p.add_member_with_value(stake_pk, stake_value);
        }
        p
    }
}
