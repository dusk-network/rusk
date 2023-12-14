// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{IterCounter, StepName};
use crate::user::stake::Stake;
use crate::{config::PROPOSAL_COMMITTEE_SIZE, user::sortition};
use node_data::bls::{PublicKey, PublicKeyBytes};

use node_data::ledger::Seed;
use num_bigint::BigInt;
use std::collections::BTreeMap;

use super::committee::Committee;

pub const DUSK: u64 = 1_000_000_000;

#[derive(Clone, Default, Debug)]
pub struct Provisioners {
    members: BTreeMap<PublicKey, Stake>,
}

impl Provisioners {
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
        let eligible_len = self.eligibles(round).count();

        (self.members.len(), eligible_len)
    }

    pub fn eligibles(
        &self,
        round: u64,
    ) -> impl Iterator<Item = (&PublicKey, &Stake)> {
        self.members
            .iter()
            .filter(move |(_, m)| m.is_eligible(round))
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

        let mut comm = CommitteeGenerator::from_provisioners(
            self,
            cfg.round,
            cfg.exclusion.as_ref(),
        );

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

    pub fn get_generator(
        &self,
        iteration: u8,
        seed: Seed,
        round: u64,
    ) -> PublicKeyBytes {
        let step = iteration.step_from_name(StepName::Proposal);
        let committee_keys = Committee::new(
            node_data::bls::PublicKey::default(),
            self,
            &sortition::Config {
                committee_size: PROPOSAL_COMMITTEE_SIZE,
                round,
                seed,
                step,
                exclusion: None,
            },
        );

        let generator = *committee_keys
            .iter()
            .next()
            .expect("committee to have 1 entry")
            .bytes();
        generator
    }
}

#[derive(Default)]
struct CommitteeGenerator<'a> {
    members: BTreeMap<&'a PublicKey, Stake>,
}

impl<'a> CommitteeGenerator<'a> {
    fn from_provisioners(
        provisioners: &'a Provisioners,
        round: u64,
        exclusion: Option<&PublicKeyBytes>,
    ) -> Self {
        let eligibles = provisioners
            .eligibles(round)
            .map(|(p, stake)| (p, stake.clone()));

        let members = match exclusion {
            None => BTreeMap::from_iter(eligibles),
            Some(excluded) => {
                let eligibles =
                    eligibles.filter(|(p, _)| p.bytes() != excluded);
                BTreeMap::from_iter(eligibles)
            }
        };

        if members.is_empty() {
            // This is the edge case when there is only 1 active provisioner.
            // Handling it just for single node cluster scenario
            let eligibles = provisioners
                .eligibles(round)
                .map(|(p, stake)| (p, stake.clone()));

            Self {
                members: BTreeMap::from_iter(eligibles),
            }
        } else {
            Self { members }
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
