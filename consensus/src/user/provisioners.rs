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

#[derive(Clone, Default, Debug)]
pub struct Provisioners {
    members: BTreeMap<PublicKey, Stake>,
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
        self.members.entry(pubkey_bls).or_insert_with(|| stake);
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

        let mut comm = CommitteeGenerator::from_provisioners(self, cfg.round);

        let mut total_amount_stake =
            BigInt::from(comm.calc_total_eligible_weight());

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
            match comm.extract_and_subtract_member(score) {
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
}

#[derive(Default)]
struct CommitteeGenerator<'a> {
    members: BTreeMap<&'a PublicKey, Stake>,
}

impl<'a> CommitteeGenerator<'a> {
    fn from_provisioners(provisioners: &'a Provisioners, round: u64) -> Self {
        let provs = provisioners.members.iter().filter_map(|(p, stake)| {
            stake.is_eligible(round).then_some((p, stake.clone()))
        });
        Self {
            members: BTreeMap::from_iter(provs),
        }
    }

    /// Sums up the total weight of all **eligible** stakes
    fn calc_total_eligible_weight(&self) -> u64 {
        self.members.values().map(|m| m.value()).sum()
    }

    fn extract_and_subtract_member(
        &mut self,
        mut score: BigInt,
    ) -> Option<(PublicKey, BigInt)> {
        if self.members.is_empty() {
            return None;
        }

        loop {
            for (&pk, stake) in self.members.iter_mut() {
                let total_stake = BigInt::from(stake.value());
                if total_stake >= score {
                    // Subtract 1 DUSK from the value extracted and rebalance
                    // accordingly.
                    let subtracted_stake = BigInt::from(stake.subtract(DUSK));

                    return Some((pk.clone(), subtracted_stake));
                }

                score -= total_stake;
            }
        }
    }
}
