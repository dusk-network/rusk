// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::sortition;
use crate::user::stake::Stake;
use hex::ToHex;
use num_bigint::BigInt;
use std::collections::BTreeMap;

pub const DUSK: u64 = 100_000_000;
pub const RAW_PUBLIC_BLS_SIZE: usize = 193;
pub const PUBLIC_BLS_SIZE: usize = 96;

// TODO: We should use dusk_bls12_381_sign::PublicKey instead.
#[derive(Eq, PartialEq, Clone, Copy, Debug, Ord, PartialOrd)]
pub struct PublicKey([u8; PUBLIC_BLS_SIZE]);

impl PublicKey {
    pub fn new(input: [u8; PUBLIC_BLS_SIZE]) -> Self {
        Self(input)
    }

    pub fn encode_short_hex(&self) -> String {
        let mut hex = self.0.as_slice().encode_hex::<String>();
        hex.truncate(16);
        hex
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Member {
    // stake and eligibility flag
    stakes: Vec<(Stake, bool)>,
    pubkey_bls: PublicKey,
    raw_pubkey_bls: [u8; RAW_PUBLIC_BLS_SIZE],
}

impl Member {
    pub fn new(pubkey_bls: PublicKey) -> Self {
        // TODO: let raw_pubkey_bls = pubkey_bls.0.to_raw_bytes();
        let raw_pubkey_bls = [0; RAW_PUBLIC_BLS_SIZE];
        Self {
            stakes: vec![],
            pubkey_bls,
            raw_pubkey_bls,
        }
    }

    pub fn get_public_key(&self) -> PublicKey {
        self.pubkey_bls
    }

    // AddStake appends a stake to the stake set with eligible_flag=false.
    pub fn add_stake(&mut self, stake: Stake) {
        self.stakes.push((stake, false));
    }

    pub fn update_eligibility_flag(&mut self, round: u64) {
        for stake in self.stakes.iter_mut() {
            stake.1 = stake.0.eligible_since <= round;
        }
    }

    pub fn subtract_from_stake(&mut self, value: u64) -> u64 {
        for stake in self.stakes.iter_mut() {
            let stake_val = stake.0.intermediate_value;
            if stake_val > 0 {
                if stake_val < value {
                    stake.0.intermediate_value = 0;
                    return stake_val;
                }
                stake.0.intermediate_value -= value;
                return value;
            }
        }

        0
    }

    pub fn restore_intermediate_value(&mut self) {
        for stake in self.stakes.iter_mut() {
            stake.0.restore_intermediate_value();
        }
    }

    fn get_total_eligible_stake(&self) -> BigInt {
        let mut total: u64 = 0;
        for stake in self.stakes.iter() {
            if stake.1 {
                total += stake.0.intermediate_value;
            }
        }

        BigInt::from(total)
    }
}

impl Default for PublicKey {
    #[inline]
    fn default() -> PublicKey {
        PublicKey([0; PUBLIC_BLS_SIZE])
    }
}

impl Default for Member {
    #[inline]
    fn default() -> Member {
        Member {
            stakes: vec![],
            pubkey_bls: PublicKey::default(),
            raw_pubkey_bls: [0; RAW_PUBLIC_BLS_SIZE],
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Provisioners {
    members: BTreeMap<PublicKey, Member>,
}

impl Provisioners {
    pub fn new() -> Self {
        Self {
            members: BTreeMap::new(),
        }
    }

    pub fn add_member(
        &mut self,
        pubkey_bls: PublicKey,
        value: u64,
        reward: u64,
        eligible_since: u64,
    ) {
        self.members
            .entry(pubkey_bls)
            .or_insert_with(|| Member::new(pubkey_bls))
            .add_stake(Stake::new(value, reward, eligible_since));
    }

    pub fn add_member_with_value(&mut self, pubkey_bls: PublicKey, value: u64) {
        self.add_member(pubkey_bls, value, 0, 0)
    }

    // update_eligibility_flag enables or disables stakes depending on specified round.
    pub fn update_eligibility_flag(&mut self, round: u64) {
        for m in self.members.iter_mut() {
            m.1.update_eligibility_flag(round)
        }
    }

    // get_eligible_size returns how many provisioners are active on the current round.
    // This function is used to determine the correct committee size for
    // sortition.
    pub fn get_eligible_size(&self, max_size: usize) -> usize {
        let mut size = 0;
        for m in self.members.iter() {
            for s in m.1.stakes.iter() {
                if s.1 {
                    size += 1;
                    break;
                }
            }

            if size >= max_size {
                return max_size;
            }
        }

        size
    }

    // create_committee runs the deterministic sortition function, which determines
    // who will be in the committee for a given step and round
    pub fn create_committee(&mut self, cfg: &sortition::Config) -> Vec<PublicKey> {
        let mut committee: Vec<PublicKey> = vec![];
        let committee_size = self.get_eligible_size(cfg.max_committee_size);

        // Restore intermediate value of all stakes.
        for m in self.members.iter_mut() {
            m.1.restore_intermediate_value();
        }

        let mut total_amount_stake = BigInt::from(self.calc_total_eligible_weight());

        let mut counter: i32 = 0;
        loop {
            if total_amount_stake.eq(&BigInt::from(0)) || committee.len() == committee_size {
                break;
            }

            // 1. Compute n ← H(seed ∣∣ round ∣∣ step ∣∣ counter)
            let hash = sortition::create_sortition_hash(cfg, counter);
            counter += 1;

            // 2. Compute d ← n mod s
            let score = sortition::generate_sortition_score(hash, &total_amount_stake);

            // NB: The public key can be extracted multiple times per committee.
            match self.extract_and_subtract_member(&score) {
                Some(m) => {
                    // append the public key to the committee set.
                    committee.push(m.0);

                    let subtracted_stake = m.1;
                    if total_amount_stake > subtracted_stake {
                        total_amount_stake -= subtracted_stake;
                    } else {
                        total_amount_stake = BigInt::from(0);
                    }
                }
                None => panic!("invalid score"),
            }
        }

        committee
    }

    // calc_total_eligible_weight sums up the total weight of all **eligible** stakes
    fn calc_total_eligible_weight(&self) -> u64 {
        let mut total_weight = 0;
        for m in self.members.iter() {
            for s in m.1.stakes.iter() {
                // Add stake value to total_weight only if it is eligible.
                if s.1 {
                    total_weight += s.0.intermediate_value;
                }
            }
        }

        total_weight
    }

    // get_active_stakes_num returns the count of all enabled stakes.
    #[allow(unused)]
    fn get_active_stakes_num(&self) -> usize {
        let mut size: usize = 0;
        for m in self.members.iter() {
            for s in m.1.stakes.iter() {
                if s.1 {
                    size += 1;
                }
            }
        }

        size
    }

    fn extract_and_subtract_member(&mut self, s: &BigInt) -> Option<(PublicKey, BigInt)> {
        let mut score = s.clone();

        if self.members.is_empty() {
            return None;
        }

        loop {
            for m in self.members.iter_mut() {
                let total_stake = m.1.get_total_eligible_stake();
                if total_stake >= score {
                    // Subtract 1 DUSK from the value extracted and rebalance accordingly.
                    let subtracted_stake = BigInt::from(m.1.subtract_from_stake(DUSK));

                    return Some((m.1.get_public_key(), subtracted_stake));
                }

                score -= total_stake;
            }
        }
    }
}

impl IntoIterator for Provisioners {
    type Item = (PublicKey, Member);
    type IntoIter = std::collections::btree_map::IntoIter<PublicKey, Member>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.into_iter()
    }
}
