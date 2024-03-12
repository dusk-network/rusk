// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::sortition;
use crate::user::stake::Stake;
use node_data::bls::{PublicKey, PublicKeyBytes};
use node_data::StepName;

use node_data::ledger::Seed;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use std::mem;

use super::committee::Committee;

pub const DUSK: u64 = 1_000_000_000;
const MINIMUM_STAKE: u64 = 1_000 * DUSK;

#[derive(Clone, Debug)]
pub struct Provisioners {
    members: BTreeMap<PublicKey, Stake>,
}

#[derive(Clone, Debug)]
pub struct ContextProvisioners {
    current: Provisioners,
    prev: Option<Provisioners>,
}

impl ContextProvisioners {
    pub fn new(current: Provisioners) -> Self {
        Self {
            current,
            prev: None,
        }
    }
    pub fn current(&self) -> &Provisioners {
        &self.current
    }
    pub fn to_current(&self) -> Provisioners {
        self.current.clone()
    }
    pub fn prev(&self) -> &Provisioners {
        self.prev.as_ref().unwrap_or(&self.current)
    }
    /// Swap `self.current` and `self.prev` and update `self.current` with [new]
    pub fn update_and_swap(&mut self, mut new: Provisioners) {
        mem::swap(&mut self.current, &mut new);

        // `new` has been swapped, and now hold the previous `self.current`
        self.prev = Some(new);
    }

    pub fn remove_previous(&mut self) {
        self.prev = None;
    }

    pub fn set_previous(&mut self, prev: Provisioners) {
        self.prev = Some(prev);
    }

    /// Change `self.current` with [new] and set `self.prev` to [None]
    pub fn update(&mut self, new: Provisioners) {
        self.current = new;
        self.prev = None;
    }
}

impl Provisioners {
    pub fn empty() -> Self {
        Self {
            members: BTreeMap::default(),
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

    pub fn replace_stake(
        &mut self,
        pubkey_bls: PublicKey,
        stake: Stake,
    ) -> Option<Stake> {
        self.members.insert(pubkey_bls, stake)
    }

    pub fn remove_stake(&mut self, pubkey_bls: &PublicKey) -> Option<Stake> {
        self.members.remove(pubkey_bls)
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
        self.members.iter().filter(move |(_, m)| {
            m.is_eligible(round) && m.value() >= MINIMUM_STAKE
        })
    }

    /// Runs the deterministic sortition algorithm which determines the
    /// committee members for a given round, step and seed.
    ///
    /// Returns a vector of provisioners public keys.
    pub(crate) fn create_committee(
        &self,
        cfg: &sortition::Config,
    ) -> Vec<PublicKey> {
        let committee_size = cfg.committee_size();
        let mut extracted: Vec<PublicKey> = Vec::with_capacity(committee_size);

        let mut comm = CommitteeGenerator::from_provisioners(
            self,
            cfg.round(),
            cfg.exclusion(),
        );

        let mut total_weight = comm.total_weight().into();

        while extracted.len() != committee_size {
            let counter = extracted.len() as u32;

            // 1. Compute n ← H(seed ∣∣ step ∣∣ counter)
            let hash = sortition::create_sortition_hash(cfg, counter);

            // 2. Compute d ← n mod s
            let score =
                sortition::generate_sortition_score(hash, &total_weight);

            // NB: The public key can be extracted multiple times per committee.
            match comm.extract_and_subtract_member(score) {
                Some((pk, subtracted_stake)) => {
                    // append the public key to the committee set.
                    extracted.push(pk);

                    if total_weight > subtracted_stake {
                        total_weight -= subtracted_stake;
                    } else {
                        break;
                    }
                }
                None => panic!("invalid score"),
            }
        }

        extracted
    }

    pub fn get_generator(
        &self,
        iteration: u8,
        seed: Seed,
        round: u64,
    ) -> PublicKeyBytes {
        let cfg = sortition::Config::new(
            seed,
            round,
            iteration,
            StepName::Proposal,
            None,
        );
        let committee_keys = Committee::new(self, &cfg);

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

    /// Sums up the total weight of all stakes
    fn total_weight(&self) -> u64 {
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
