// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::mem;

use dusk_core::dusk;
use dusk_core::stake::DEFAULT_MINIMUM_STAKE;
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
    /// If the provisioner already exists, no action is performed.
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

    /// Subtract `amount` from a staker, returning the stake left
    ///
    /// Return None if the entry was not found or `amount` is higher than
    /// current stake
    pub fn sub_stake(
        &mut self,
        pubkey_bls: &PublicKey,
        amount: u64,
    ) -> Option<u64> {
        let stake = self.members.get_mut(pubkey_bls)?;
        if stake.value() < amount {
            None
        } else {
            stake.subtract(amount);
            let left = stake.value();
            if left == 0 {
                self.members.remove(pubkey_bls);
            }
            Some(left)
        }
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
            m.is_eligible(round) && m.value() >= DEFAULT_MINIMUM_STAKE
        })
    }

    /// Runs the deterministic sortition algorithm which determines the
    /// committee members for a given round, step and seed.
    ///
    /// Returns the committee as a list of the extracted provisioners public
    /// keys, where keys can have repetitions.
    pub(crate) fn create_committee(
        &self,
        cfg: &sortition::Config,
    ) -> Vec<PublicKey> {
        let committee_credits = cfg.committee_credits();
        // List of the extracted members.
        // Note: members extracted multiple times will appear multiple times in
        // the list
        let mut committee: Vec<PublicKey> =
            Vec::with_capacity(committee_credits);

        let mut comm_gen =
            CommitteeGenerator::new(self, cfg.round(), cfg.exclusion());

        let mut eligible_weight = comm_gen.eligible_weight().into();

        while committee.len() != committee_credits {
            let credit_index = committee.len() as u32;

            // Compute sortition hash
            // hash = H(seed ∣∣ step ∣∣ index)
            let hash = sortition::create_sortition_hash(cfg, credit_index);

            // Compute sortition score
            // score = hash % eligible_weight
            let score =
                sortition::generate_sortition_score(hash, &eligible_weight);

            // Extract the committee member.
            // Note: eligible provisioners can be extracted multiple times for
            // the same committee
            let (prov_pk, prov_weight) = comm_gen.extract_member(score);
            // Add the extracted member to the committee
            committee.push(prov_pk);

            if eligible_weight > prov_weight {
                eligible_weight -= prov_weight;
            } else {
                break;
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
    // Provisioners eligible for the committee
    eligibles: BTreeMap<&'a PublicKey, Stake>,
}

impl<'a> CommitteeGenerator<'a> {
    /// Creates a [`CommitteeGenerator`] from the provisioner set.
    ///
    /// # Arguments
    /// * `provisioners` - the current list of provisioners
    /// * `round` - the round of the extraction (to determine eligibility)
    /// * `exclusion_list` - list of provisioners to exclude from extraction
    fn new(
        provisioners: &'a Provisioners,
        round: u64,
        exclusion_list: &Vec<PublicKeyBytes>,
    ) -> Self {
        // Get provisioners eligible at round `round`
        let eligible_set: Vec<_> = provisioners
            .eligibles(round)
            .map(|(pk, stake)| (pk, stake.clone()))
            .collect();

        let num_eligibles = eligible_set.len();
        let eligible_set = eligible_set.into_iter();

        // Set `eligibles` to  the eligible set minus the exclusion list
        let eligibles = if num_eligibles > 1 {
            let eligible_iter = eligible_set;
            match exclusion_list.len() {
                0 => BTreeMap::from_iter(eligible_iter),
                _ => {
                    let filtered_eligibles = eligible_iter.filter(|(p, _)| {
                        !exclusion_list
                            .iter()
                            .any(|excluded| excluded == p.bytes())
                    });
                    BTreeMap::from_iter(filtered_eligibles)
                }
            }
        } else {
            // If only one provisioner is eligible, we always include it
            BTreeMap::from_iter(eligible_set)
        };

        Self { eligibles }
    }

    /// Sums up the total weight of all eligible provisioners
    fn eligible_weight(&self) -> u64 {
        self.eligibles.values().map(|m| m.value()).sum()
    }

    /// Extracts a member from `eligibles` given a Sortition score.
    ///
    /// At the beginning of the extraction, each provisioner has a weight equal
    /// to its stake. Each time a provisioner is extracted, its weight is
    /// reduced by 1 DUSK to decrease its probability of being extracted.
    ///
    /// # Arguments
    /// * `score` - the Sortition score for the extraction
    ///
    /// # Output
    /// * The extracted stake [`PublicKey`]
    /// * The remaining stake weight after the extraction
    fn extract_member(&mut self, mut score: BigInt) -> (PublicKey, BigInt) {
        if self.eligibles.is_empty() {
            panic!("No eligible provisioners to extract for the committee");
        }

        loop {
            // Loop over eligible provisioners
            for (&provisioner, provisioner_weight) in self.eligibles.iter_mut()
            {
                // Set the initial provisioner's weight to the stake's value
                let weight = BigInt::from(provisioner_weight.value());

                // If the provisioner's weight is higher than the score, extract
                // the provisioner and reduce its weight
                if weight >= score {
                    // Subtract 1 DUSK from the extracted stake's weight
                    let new_weight =
                        BigInt::from(provisioner_weight.subtract(DUSK));

                    return (provisioner.clone(), new_weight);
                }

                // Otherwise, decrease the score and move to the next
                // provisioner
                score -= weight;
            }
        }
    }
}
