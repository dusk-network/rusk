// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::consensus::CONSENSUS_QUORUM_THRESHOLD;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;

use crate::util::cluster::Cluster;
use crate::util::pubkey::PublicKey;
use std::collections::{BTreeMap, HashMap};
use std::mem;

#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct Committee {
    members: BTreeMap<PublicKey, usize>,
    this_member_key: PublicKey,
    cfg: sortition::Config,
    total: usize,
}

#[allow(unused)]
impl Committee {
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &mut Provisioners,
        cfg: sortition::Config,
    ) -> Self {
        provisioners.update_eligibility_flag(cfg.round);
        // Generate committee using deterministic sortition.
        let res = provisioners.create_committee(&cfg);

        // Turn the raw vector into a hashmap where we map a pubkey to its occurrences.
        let mut committee = Self {
            members: BTreeMap::new(),
            this_member_key: pubkey_bls,
            cfg,
            total: 0,
        };

        for member_key in res.as_slice() {
            *committee.members.entry(*member_key).or_insert(0) += 1;
            committee.total += 1;
        }

        debug_assert!(committee.total == provisioners.get_eligible_size(cfg.max_committee_size));

        committee
    }

    pub fn is_member(&self, pubkey_bls: PublicKey) -> bool {
        self.members.contains_key(&pubkey_bls)
    }

    pub fn am_member(&self) -> bool {
        self.is_member(self.this_member_key)
    }

    // get_my_pubkey returns this provisioner BLS public key.
    pub fn get_my_pubkey(&self) -> PublicKey {
        self.this_member_key
    }

    pub fn votes_for(&self, pubkey_bls: PublicKey) -> Option<&usize> {
        self.members.get(&pubkey_bls)
    }

    // get_occurrences returns values in a vec
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.members.clone().into_values().collect()
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }

    pub fn quorum(&self) -> usize {
        let size = self.total as f64;
        (size * CONSENSUS_QUORUM_THRESHOLD).ceil() as usize
    }

    pub fn bits(&self, voters: &Cluster<PublicKey>) -> u64 {
        let mut bits: u64 = 0;

        debug_assert!(self.members.len() <= mem::size_of_val(&bits) * 8);

        let mut pos = 0;
        for item in voters.0.iter() {
            for (pos, member) in self.members.iter().enumerate() {
                if member.0 == item.0 {
                    bits |= 1 << pos; // flip the i-th bit to 1
                    break;
                }
            }
        }

        bits
    }

    /// intersect the bit representation of a VotingCommittee subset with the whole VotingCommittee set.
    pub fn intersect(&self, bitset: u64) -> Cluster<PublicKey> {
        if bitset == 0 {
            return Cluster::<PublicKey>::new();
        }

        let mut a = Cluster::<PublicKey>::new();
        for (pos, member) in self.members.iter().enumerate() {
            if ((bitset >> pos) & 1) != 0 {
                a.set_weight(member.0, *member.1);
            }
        }
        a
    }

    pub fn total_occurrences(&self, voters: &Cluster<PublicKey>) -> usize {
        let mut total = 0;
        for item in voters.0.iter() {
            match self.votes_for(*item.0) {
                Some(weight) => {
                    total += *weight;
                }
                None => {}
            };
        }

        total
    }
}

/// CommitteeSet implements a cache of generated committees so that they can be reused.
/// This is useful in Agreement step where messages from different steps per a single round are processed.
///  A committee is uniquely represented by sortition::Config.
pub struct CommitteeSet {
    committees: HashMap<sortition::Config, Committee>,
    provisioners: Provisioners,
    this_member_key: PublicKey,
}

impl CommitteeSet {
    pub fn new(pubkey: PublicKey, provisioners: Provisioners) -> Self {
        CommitteeSet {
            provisioners,
            committees: HashMap::new(),
            this_member_key: pubkey,
        }
    }

    pub fn is_member(&mut self, pubkey: PublicKey, cfg: sortition::Config) -> bool {
        self.get_or_create(cfg).is_member(pubkey)
    }

    pub fn votes_for(&mut self, pubkey: PublicKey, cfg: sortition::Config) -> Option<&usize> {
        self.get_or_create(cfg).votes_for(pubkey)
    }

    pub fn quorum(&mut self, cfg: sortition::Config) -> usize {
        self.get_or_create(cfg).quorum()
    }

    pub fn intersect(&mut self, bitset: u64, cfg: sortition::Config) -> Cluster<PublicKey> {
        self.get_or_create(cfg).intersect(bitset)
    }

    pub fn total_occurrences(
        &mut self,
        voters: &Cluster<PublicKey>,
        cfg: sortition::Config,
    ) -> usize {
        self.get_or_create(cfg).total_occurrences(voters)
    }

    pub fn get_provisioners(&self) -> &Provisioners {
        &self.provisioners
    }

    fn get_or_create(&mut self, cfg: sortition::Config) -> &Committee {
        self.committees
            .entry(cfg)
            .or_insert_with(|| Committee::new(self.this_member_key, &mut self.provisioners, cfg))
    }
}
