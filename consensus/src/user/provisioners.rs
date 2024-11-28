// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::mem;

use execution_core::dusk;
use execution_core::stake::MINIMUM_STAKE;
use node_data::bls::{PublicKey, PublicKeyBytes};
use node_data::ledger::Seed;
use node_data::StepName;
use num_bigint::BigInt;

use super::committee::Committee;
use crate::user::sortition;
use crate::user::stake::Stake;

pub const DUSK: u64 = dusk(1.0);

#[derive(Clone, Debug)]
pub struct Provisioners {
    members: BTreeMap<PublicKey, Stake>,
}

impl Provisioners {
    pub fn iter(&self) -> impl Iterator<Item = (&PublicKey, &Stake)> {
        self.members.iter()
    }
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
    /// Swap `self.current` and `self.prev` and update `self.current` with `new`
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

    /// Change `self.current` with `new` and set `self.prev` to [None]
    pub fn update(&mut self, new: Provisioners) {
        self.current = new;
        self.prev = None;
    }

    /// Derive the previous state of provisioners.
    ///
    /// This method takes a vector of tuples representing the previous state of
    /// each provisioner. Each tuple consists of a `PublicKey` and an
    /// optional `Stake`.
    ///
    /// If the `changes` vector is not empty, it iterates
    /// over each change, deriving the previous state of provisioners from
    /// the current state, and updates the current state accordingly.
    ///
    /// If the `changes` vector is empty, the previous state of the provisioners
    /// is considered equal to the current
    pub fn apply_changes(&mut self, changes: Vec<(PublicKey, Option<Stake>)>) {
        if !changes.is_empty() {
            let mut prev = self.to_current();
            for change in changes {
                match change {
                    (pk, None) => prev.remove_stake(&pk),
                    (pk, Some(stake)) => prev.replace_stake(pk, stake),
                };
            }
            self.set_previous(prev)
        } else {
            self.remove_previous()
        }
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

    pub fn get_member_mut(
        &mut self,
        pubkey_bls: &PublicKey,
    ) -> Option<&mut Stake> {
        self.members.get_mut(pubkey_bls)
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
        let committee_credits = cfg.committee_credits();
        let mut extracted: Vec<PublicKey> =
            Vec::with_capacity(committee_credits);

        let mut comm = CommitteeGenerator::from_provisioners(
            self,
            cfg.round(),
            cfg.exclusion(),
        );

        let mut total_weight = comm.total_weight().into();

        while extracted.len() != committee_credits {
            let counter = extracted.len() as u32;

            // 1. Compute n ← H(seed ∣∣ step ∣∣ counter)
            let hash = sortition::create_sortition_hash(cfg, counter);

            // 2. Compute d ← n mod s
            let score =
                sortition::generate_sortition_score(hash, &total_weight);

            // NB: The public key can be extracted multiple times per committee.
            let (pk, subtracted_stake) =
                comm.extract_and_subtract_member(score);
            // append the public key to the committee set.
            extracted.push(pk);

            if total_weight > subtracted_stake {
                total_weight -= subtracted_stake;
            } else {
                break;
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
            vec![],
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
        exclusion: &Vec<PublicKeyBytes>,
    ) -> Self {
        let eligibles = provisioners
            .eligibles(round)
            .map(|(p, stake)| (p, stake.clone()));

        let members = match exclusion.len() {
            0 => BTreeMap::from_iter(eligibles),
            _ => {
                let eligibles = eligibles.filter(|(p, _)| {
                    !exclusion.iter().any(|excluded| excluded == p.bytes())
                });
                BTreeMap::from_iter(eligibles)
            }
        };

        if members.is_empty() {
            // This is the edge case when there is only 1 active provisioner.
            // Handling it just for single node cluster scenario
            let eligibles = provisioners
                .eligibles(round)
                .map(|(p, stake)| (p, stake.clone()));

            let members = BTreeMap::from_iter(eligibles);

            debug_assert!(
                !members.is_empty(),
                "No provisioners are eligible for the committee"
            );

            Self { members }
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
    ) -> (PublicKey, BigInt) {
        if self.members.is_empty() {
            panic!("Cannot extract member from an empty committee");
        }

        loop {
            for (&pk, stake) in self.members.iter_mut() {
                let total_stake = BigInt::from(stake.value());
                if total_stake >= score {
                    // Subtract 1 DUSK from the value extracted and rebalance
                    // accordingly.
                    let subtracted_stake = BigInt::from(stake.subtract(DUSK));

                    return (pk.clone(), subtracted_stake);
                }

                score -= total_stake;
            }
        }
    }
}
