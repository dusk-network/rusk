// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::{BTreeMap, HashMap};
use std::{fmt, mem};

use node_data::bls::{PublicKey, PublicKeyBytes};

use super::cluster::Cluster;
use crate::config::{majority, supermajority};
use crate::user::provisioners::Provisioners;
use crate::user::sortition;

#[derive(Default, Debug, Clone)]
pub struct Committee {
    members: BTreeMap<PublicKey, usize>,
    super_majority: usize,
    majority: usize,
    excluded: Vec<PublicKeyBytes>,
}

impl Committee {
    pub fn iter(&self) -> impl Iterator<Item = &PublicKey> {
        self.members.keys()
    }
}

impl Committee {
    /// Generates a new committee from the given provisioners state and
    /// sortition config.
    ///
    /// It executes deterministic sortition algorithm.
    pub fn new(provisioners: &Provisioners, cfg: &sortition::Config) -> Self {
        // Generate committee using deterministic sortition.
        let extracted = provisioners.create_committee(cfg);
        let committee_credits = cfg.committee_credits();

        let majority = majority(committee_credits);
        let super_majority = supermajority(committee_credits);

        // Turn the raw vector into a hashmap where we map a pubkey to its
        // occurrences.
        let mut committee = Self {
            members: BTreeMap::new(),
            super_majority,
            majority,
            excluded: cfg.exclusion().clone(),
        };

        for member_key in extracted {
            *committee.members.entry(member_key).or_insert(0) += 1;
        }

        committee
    }

    pub fn excluded(&self) -> &Vec<PublicKeyBytes> {
        &self.excluded
    }

    /// Returns true if `pubkey_bls` is a member of the generated committee.
    pub fn is_member(&self, pubkey_bls: &PublicKey) -> bool {
        self.members.contains_key(pubkey_bls)
    }

    pub fn votes_for(&self, pubkey_bls: &PublicKey) -> Option<usize> {
        self.members.get(pubkey_bls).copied()
    }

    pub fn members(&self) -> &BTreeMap<PublicKey, usize> {
        &self.members
    }

    // get_occurrences returns values in a vec
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.members.values().copied().collect()
    }

    /// Returns number of unique members of the generated committee.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Returns target supermajority quorum for the generated committee.
    pub fn super_majority_quorum(&self) -> usize {
        self.super_majority
    }

    /// Returns target majority quorum for the generated committee.
    pub fn majority_quorum(&self) -> usize {
        self.majority
    }

    pub fn bits(&self, voters: &Cluster<PublicKey>) -> u64 {
        let mut bits: u64 = 0;

        debug_assert!(self.members.len() <= mem::size_of_val(&bits) * 8);

        for (pos, (member_pk, _)) in self.members.iter().enumerate() {
            if voters.contains_key(member_pk) {
                bits |= 1 << pos; // flip the i-th bit to 1
            }
        }

        bits
    }

    /// Intersects the bit representation of a VotingCommittee subset with the
    /// whole VotingCommittee set.
    pub fn intersect(&self, bitset: u64) -> Cluster<PublicKey> {
        if bitset == 0 {
            return Cluster::<PublicKey>::new();
        }

        let mut a = Cluster::new();
        for (pos, (member_pk, weight)) in self.members.iter().enumerate() {
            if ((bitset >> pos) & 1) != 0 {
                a.add(member_pk, *weight);
            }
        }
        a
    }

    pub fn total_occurrences(&self, voters: &Cluster<PublicKey>) -> usize {
        voters
            .iter()
            .flat_map(|(voter, _)| self.votes_for(voter))
            .sum()
    }
}

impl fmt::Display for &Committee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (pos, (member_pk, weight)) in self.members.iter().enumerate() {
            write!(f, " [{}]=pk:{} w:{},", pos, member_pk.to_bs58(), weight)?;
        }

        Ok(())
    }
}

/// Implements a cache of generated committees so that they can be reused.
#[derive(Clone)]
pub struct CommitteeSet<'p> {
    committees: HashMap<sortition::Config, Committee>,
    provisioners: &'p Provisioners,
}

impl<'p> CommitteeSet<'p> {
    pub fn new(provisioners: &'p Provisioners) -> Self {
        CommitteeSet {
            provisioners,
            committees: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, cfg: &sortition::Config) -> &Committee {
        self.committees
            .entry(cfg.clone())
            .or_insert_with_key(|config| {
                Committee::new(self.provisioners, config)
            })
    }

    pub fn get(&self, cfg: &sortition::Config) -> Option<&Committee> {
        self.committees.get(cfg)
    }

    pub fn provisioners(&self) -> &Provisioners {
        self.provisioners
    }
}
