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
pub struct Member {
    stake: Stake,
    // ephemeral value used to perform deterministic sortition
    intermediate_value: u64,
}

impl Member {
    pub fn new(stake: Stake) -> Self {
        let intermediate_value = stake.value();
        Self {
            stake,
            intermediate_value,
        }
    }

    pub fn stake(&self) -> &Stake {
        &self.stake
    }

    pub fn is_eligible(&self, round: u64) -> bool {
        self.stake.eligible_since <= round
    }

    pub fn subtract_from_stake(&mut self, value: u64) -> u64 {
        let stake_val = self.intermediate_value;
        if stake_val > 0 {
            if stake_val < value {
                self.intermediate_value = 0;
                return stake_val;
            }
            self.intermediate_value -= value;
            return value;
        }

        0
    }

    fn restore_intermediate_value(&mut self) {
        self.intermediate_value = self.stake.value();
    }

    fn get_total_eligible_stake(&self, round: u64) -> BigInt {
        if self.stake.eligible_since <= round {
            BigInt::from(self.intermediate_value)
        } else {
            BigInt::from(0u64)
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
            .or_insert_with(|| Member::new(stake));
    }

    /// Adds a new member with reward=0 and elibile_since=0.
    ///
    /// Useful for implementing unit tests.
    pub fn add_member_with_value(&mut self, pubkey_bls: PublicKey, value: u64) {
        self.add_member_with_stake(pubkey_bls, Stake::from_value(value));
    }

    // Returns a pair of count of all provisioners and count of eligible
    // provisioners for the specified round.
    pub fn get_provisioners_info(&self, round: u64) -> (usize, usize) {
        let eligible_len = self
            .members
            .iter()
            .filter(|(_, m)| m.is_eligible(round))
            .count();

        (self.members.len(), eligible_len)
    }

    /// Runs the deterministic sortition algorithm which determines the
    /// committee members for a given round, step and seed.
    ///
    /// Returns a vector of provisioners public keys.
    pub(crate) fn create_committee(
        &self,
        cfg: &sortition::Config,
    ) -> Vec<PublicKey> {
        let mut committee: Vec<PublicKey> = vec![];

        let mut provisioners = self.clone();

        // Restore intermediate value of all stakes.
        for (_, member) in provisioners.members.iter_mut() {
            member.restore_intermediate_value();
        }

        let mut total_amount_stake =
            BigInt::from(provisioners.calc_total_eligible_weight(cfg.round));

        let mut counter: u32 = 0;
        loop {
            if total_amount_stake.eq(&BigInt::from(0))
                || committee.len() == cfg.committee_size
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
            match provisioners.extract_and_subtract_member(score, cfg.round) {
                Some((pk, value)) => {
                    // append the public key to the committee set.
                    committee.push(pk);

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
    fn calc_total_eligible_weight(&self, round: u64) -> u64 {
        self.members
            .values()
            .filter_map(|m| {
                m.is_eligible(round).then_some(m.intermediate_value)
            })
            .sum()
    }

    fn extract_and_subtract_member(
        &mut self,
        mut score: BigInt,
        round: u64,
    ) -> Option<(PublicKey, BigInt)> {
        if self.members.is_empty() {
            return None;
        }

        loop {
            for (pk, member) in self.members.iter_mut() {
                let total_stake = member.get_total_eligible_stake(round);
                if total_stake >= score {
                    // Subtract 1 DUSK from the value extracted and rebalance
                    // accordingly.
                    let subtracted_stake =
                        BigInt::from(member.subtract_from_stake(DUSK));

                    return Some((pk.clone(), subtracted_stake));
                }

                score -= total_stake;
            }
        }
    }
}
