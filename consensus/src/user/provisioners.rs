// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::sortition;
use crate::user::stake::Stake;
use node_data::bls::PublicKey;

use num_bigint::BigInt;
use std::collections::BTreeMap;

pub const DUSK: u64 = 1_000_000_000;

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct Member {
    /// Vector of pairs (stake and eligibility flag)
    stakes: Vec<(Stake, bool)>,
    pubkey_bls: PublicKey,
}

impl Member {
    pub fn new(pubkey_bls: PublicKey) -> Self {
        Self {
            stakes: vec![],
            pubkey_bls,
        }
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.pubkey_bls
    }

    // AddStake appends a stake to the stake set with eligible_flag=false.
    pub fn add_stake(&mut self, stake: Stake) {
        self.stakes.push((stake, false));
    }

    pub fn update_eligibility_flag(&mut self, round: u64) {
        for (stake, eligible) in self.stakes.iter_mut() {
            *eligible = stake.eligible_since <= round;
        }
    }

    pub fn subtract_from_stake(&mut self, value: u64) -> u64 {
        for (stake, _) in self.stakes.iter_mut() {
            let stake_val = stake.intermediate_value;
            if stake_val > 0 {
                if stake_val < value {
                    stake.intermediate_value = 0;
                    return stake_val;
                }
                stake.intermediate_value -= value;
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
        for (stake, eligible) in self.stakes.iter() {
            if *eligible {
                total += stake.intermediate_value;
            }
        }

        BigInt::from(total)
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

    /// Adds a provisioner with stake.
    ///
    /// It appends the stake if the given provisioner already exists.
    pub fn add_member_with_stake(
        &mut self,
        pubkey_bls: PublicKey,
        stake: Stake,
    ) {
        self.members
            .entry(pubkey_bls)
            .or_insert_with_key(|key| Member::new(key.clone()))
            .add_stake(stake);
    }

    /// Adds a new member with reward=0 and elibile_since=0.
    ///
    /// Useful for implementing unit tests.
    pub fn add_member_with_value(&mut self, pubkey_bls: PublicKey, value: u64) {
        self.add_member_with_stake(pubkey_bls, Stake::new(value, 0, 0));
    }

    /// Turns on/off elibility flag of stakes for a given round.
    pub fn update_eligibility_flag(&mut self, round: u64) {
        self.members
            .values_mut()
            .for_each(|m| m.update_eligibility_flag(round));
    }

    /// Returns number of provisioners that owns at least one eligibile stake.
    pub fn get_eligible_size(&self, max_size: usize) -> usize {
        self.members
            .iter()
            .filter(|(_, m)| m.stakes.iter().any(|(_, elegible)| *elegible))
            .take(max_size)
            .count()
    }

    /// Returns a member of Provisioner list by public key.
    pub fn get_member(&self, key: &PublicKey) -> Option<&Member> {
        self.members.get(key)
    }

    /// Runs the deterministic sortition algorithm which determines the committee members for a given round, step and seed.
    ///
    /// Returns a vector of provisioners public keys.
    pub fn create_committee(
        &mut self,
        cfg: &sortition::Config,
    ) -> Vec<PublicKey> {
        let mut committee: Vec<PublicKey> = vec![];
        let committee_size = self.get_eligible_size(cfg.max_committee_size);

        // Restore intermediate value of all stakes.
        for (_, member) in self.members.iter_mut() {
            member.restore_intermediate_value();
        }

        let mut total_amount_stake =
            BigInt::from(self.calc_total_eligible_weight());

        let mut counter: u32 = 0;
        loop {
            if total_amount_stake.eq(&BigInt::from(0))
                || committee.len() == committee_size
            {
                break;
            }

            // 1. Compute n ← H(seed ∣∣ round ∣∣ step ∣∣ counter)
            let hash = sortition::create_sortition_hash(cfg, counter);
            counter += 1;

            // 2. Compute d ← n mod s
            let score =
                sortition::generate_sortition_score(hash, &total_amount_stake);

            // NB: The public key can be extracted multiple times per committee.
            match self.extract_and_subtract_member(score) {
                Some((pk, value)) => {
                    // append the public key to the committee set.
                    committee.push(pk.clone());

                    let subtracted_stake = value;
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

    /// Sums up the total weight of all **eligible** stakes
    fn calc_total_eligible_weight(&self) -> u64 {
        self.members
            .values()
            .flat_map(|m| &m.stakes)
            .filter_map(|(stake, eligible)| {
                eligible.then(|| stake.intermediate_value)
            })
            .sum()
    }

    fn extract_and_subtract_member(
        &mut self,
        mut score: BigInt,
    ) -> Option<(PublicKey, BigInt)> {
        if self.members.is_empty() {
            return None;
        }

        loop {
            for member in self.members.values_mut() {
                let total_stake = member.get_total_eligible_stake();
                if total_stake >= score {
                    // Subtract 1 DUSK from the value extracted and rebalance accordingly.
                    let subtracted_stake =
                        BigInt::from(member.subtract_from_stake(DUSK));

                    return Some((
                        member.public_key().clone(),
                        subtracted_stake,
                    ));
                }

                score -= total_stake;
            }
        }
    }
}
