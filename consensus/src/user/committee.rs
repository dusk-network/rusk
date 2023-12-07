// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::user::provisioners::Provisioners;
use crate::user::sortition;

use super::cluster::Cluster;
use crate::config;
use node_data::bls::PublicKey;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::mem;

#[derive(Default, Debug, Clone)]
pub struct Committee {
    members: BTreeMap<PublicKey, usize>,
    this_member_key: PublicKey,
    quorum: usize,
    nil_quorum: usize,
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
    ///
    /// # Arguments
    /// * `pubkey_bls` - This is the BLS public key of the (this node)
    ///   provisioner running the consensus. It is mainly used in `am_member`
    ///   method.
    pub fn new(
        pubkey_bls: PublicKey,
        provisioners: &Provisioners,
        cfg: &sortition::Config,
    ) -> Self {
        // Generate committee using deterministic sortition.
        let res = provisioners.create_committee(cfg);

        let quorum = (cfg.committee_size as f64
            * config::CONSENSUS_QUORUM_THRESHOLD)
            .ceil() as usize;
        let nil_quorum = cfg.committee_size - quorum + 1;

        // Turn the raw vector into a hashmap where we map a pubkey to its
        // occurrences.
        let mut committee = Self {
            members: BTreeMap::new(),
            this_member_key: pubkey_bls,
            nil_quorum,
            quorum,
        };

        for member_key in res {
            *committee.members.entry(member_key).or_insert(0) += 1;
        }

        committee
    }

    /// Returns true if `pubkey_bls` is a member of the generated committee.
    pub fn is_member(&self, pubkey_bls: &PublicKey) -> bool {
        self.members.contains_key(pubkey_bls)
    }

    /// Returns true if `my pubkey` is a member of the generated committee.
    pub fn am_member(&self) -> bool {
        self.is_member(&self.this_member_key)
    }

    /// Returns this provisioner BLS public key.
    pub fn get_my_pubkey(&self) -> &PublicKey {
        &self.this_member_key
    }

    pub fn votes_for(&self, pubkey_bls: &PublicKey) -> Option<usize> {
        self.members.get(pubkey_bls).copied()
    }

    // get_occurrences returns values in a vec
    pub fn get_occurrences(&self) -> Vec<usize> {
        self.members.clone().into_values().collect()
    }

    /// Returns number of unique members of the generated committee.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Returns target quorum for the generated committee.
    pub fn quorum(&self) -> usize {
        self.quorum
    }

    /// Returns target NIL quorum for the generated committee.
    pub fn nil_quorum(&self) -> usize {
        self.nil_quorum
    }

    pub fn bits(&self, voters: &Cluster<PublicKey>) -> u64 {
        let mut bits: u64 = 0;

        debug_assert!(self.members.len() <= mem::size_of_val(&bits) * 8);

        for (pk, _) in voters.iter() {
            for (pos, (member_pk, _)) in self.members.iter().enumerate() {
                if member_pk.eq(pk) {
                    bits |= 1 << pos; // flip the i-th bit to 1
                    break;
                }
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
                a.set_weight(member_pk, *weight);
            }
        }
        a
    }

    pub fn total_occurrences(&self, voters: &Cluster<PublicKey>) -> usize {
        let mut total = 0;
        for (item_pk, _) in voters.iter() {
            if let Some(weight) = self.votes_for(item_pk) {
                total += weight;
            };
        }

        total
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
///
/// This is useful in Agreement step where messages from different steps per a
/// single round are concurrently processed. A committee is uniquely represented
/// by sortition::Config.
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

    pub fn is_member(
        &mut self,
        pubkey: &PublicKey,
        cfg: &sortition::Config,
    ) -> bool {
        self.get_or_create(cfg).is_member(pubkey)
    }

    /// Returns number of all unique public keys
    pub fn get_unique_members(&self) -> usize {
        let mut merged = HashSet::new();
        self.committees.iter().for_each(|(_, committee)| {
            committee.members.iter().for_each(|(m, s)| {
                if *s > 0 {
                    merged.insert(m.bytes());
                }
            });
        });

        merged.len()
    }

    pub fn votes_for(
        &mut self,
        pubkey: &PublicKey,
        cfg: &sortition::Config,
    ) -> Option<usize> {
        self.get_or_create(cfg).votes_for(pubkey)
    }

    pub fn quorum(&mut self, cfg: &sortition::Config) -> usize {
        self.get_or_create(cfg).quorum()
    }
    pub fn nil_quorum(&mut self, cfg: &sortition::Config) -> usize {
        self.get_or_create(cfg).nil_quorum()
    }

    pub fn intersect(
        &mut self,
        bitset: u64,
        cfg: &sortition::Config,
    ) -> Cluster<PublicKey> {
        self.get_or_create(cfg).intersect(bitset)
    }

    pub fn total_occurrences(
        &mut self,
        voters: &Cluster<PublicKey>,
        cfg: &sortition::Config,
    ) -> usize {
        self.get_or_create(cfg).total_occurrences(voters)
    }

    pub fn get_provisioners(&self) -> &Provisioners {
        &self.provisioners
    }

    pub fn bits(
        &mut self,
        voters: &Cluster<PublicKey>,
        cfg: &sortition::Config,
    ) -> u64 {
        self.get_or_create(cfg).bits(voters)
    }

    fn get_or_create(&mut self, cfg: &sortition::Config) -> &Committee {
        self.committees
            .entry(cfg.clone())
            .or_insert_with_key(|config| {
                Committee::new(
                    self.this_member_key.clone(),
                    &self.provisioners,
                    config,
                )
            })
    }
}
