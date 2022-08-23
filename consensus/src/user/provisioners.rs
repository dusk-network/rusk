// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::ConsensusError;
use dusk_bls12_381_sign::PublicKey;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// HashablePubKey satisfies Hash trait for dusk_bls12_381_sign::PublicKey.
// TODO: We can support this in dusk_bls12_381_sign instead.
#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub struct HashablePubKey(PublicKey);

impl HashablePubKey {
    pub fn new(pubkey: PublicKey) -> Self {
        Self { 0: pubkey }
    }
}

impl Hash for HashablePubKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO: to_bytes is private
        state.write(self.0.pk_t().to_raw_bytes().as_slice());
        state.finish();
    }
}

#[derive(Copy, Clone, Default, Debug)]
#[allow(unused)]
pub struct Stake {
    value: u64,
    reward: u64,
    counter: u64,
    eligibility: u64,
}

impl Stake {
    pub fn new(value: u64, reward: u64, eligibility: u64) -> Self {
        Self {
            value,
            reward,
            eligibility,
            counter: 0,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Member {
    // stake and flag enabled/disabled.
    stakes: Vec<(Stake, bool)>,
    pubkey_bls: HashablePubKey,
    raw_pubkey_bls: [u8; 193],
}

impl Member {
    pub fn new(pubkey_bls: HashablePubKey) -> Self {
        let raw_pubkey_bls = pubkey_bls.0.to_raw_bytes();
        Self {
            stakes: vec![],
            pubkey_bls,
            raw_pubkey_bls,
        }
    }

    // AddStake appends a stake to the stake set.
    pub fn add_stake(&mut self, stake: Stake) {
        self.stakes.push((stake, false));
    }

    pub fn update_eligibility_flag(&mut self, round: u64) {
        for stake in self.stakes.iter_mut() {
            stake.1 = stake.0.eligibility > round;
        }
    }

    pub fn subtract_from_stake(&mut self, value: u64) -> u64 {
        for stake in self.stakes.iter_mut() {
            let stake_val = stake.0.value;
            if stake_val > 0 {
                if stake_val < value {
                    stake.0.value = 0;
                    return stake_val;
                }
                stake.0.value -= value;
                return value;
            }
        }

        0
    }
}

impl Default for Member {
    #[inline]
    fn default() -> Member {
        Member {
            stakes: vec![],
            pubkey_bls: HashablePubKey::default(),
            raw_pubkey_bls: [0; 193],
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Provisioners {
    members: HashMap<HashablePubKey, Member>,
}

impl Provisioners {
    pub fn new() -> Self {
        Self {
            members: HashMap::new(),
        }
    }

    pub fn add_member(
        &mut self,
        pubkey_bls: HashablePubKey,
        value: u64,
        reward: u64,
        eligibility: u64,
    ) -> Option<ConsensusError> {
        let stake = Stake::new(value, reward, eligibility);

        self.members
            .entry(pubkey_bls.clone())
            .or_insert(Member::new(pubkey_bls))
            .add_stake(stake);

        None
    }

    // calc_total_active_weight sums up the total weight of all **enabled** stakes
    pub fn calc_total_active_weight(&self) -> u64 {
        let mut total_weight = 0;
        for m in self.members.iter() {
            for s in m.1.stakes.iter() {
                // Add stake value to total_weight only if it is enabled.
                // NB: a stake must be enabled/disabled accordingly at the beginning of each round.
                if s.1 {
                    total_weight += s.0.value;
                }
            }
        }

        total_weight
    }

    // get_active_stakes_num returns the count of all enabled stakes.
    pub fn get_active_stakes_num(&self) -> usize {
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

    // update_eligibility_flag enables or disables stakes depending on specified round.
    pub fn update_eligibility_flag(&mut self, round: u64) {
        for m in self.members.iter_mut() {
            m.1.update_eligibility_flag(round)
        }
    }

    // create_voting_committee runs the deterministic sortition function, which determines
    // who will be in the committee for a given step and round
    pub fn create_voting_committee(
        &self,
        _seed: [u8; 32],
        _round: u64,
        _step: u8,
        _size: usize,
    ) -> bool {
        // TODO: create_voting_committee
        true
    }
}
