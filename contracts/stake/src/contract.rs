// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_pki::StealthAddress;
use phoenix_core::Note;

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::ops::{Deref, DerefMut};

#[cfg(feature = "transaction")]
mod transaction;
use super::stake::Stake;
use super::error::Error;

/// Contract keeping track of each public key's stake.
///
/// A caller can stake Dusk, and have it attached to a public key. This stake
/// has a maturation period, after which it is considered valid and the key
/// eligible to participate in the consensus.
///
/// Rewards may be received by a public key regardless of whether they have a
/// valid stake.
#[derive(Debug, Default, Clone)]
pub struct StakeContract {
    pub(crate) stakes: BTreeMap<[u8; PublicKey::SIZE], Stake>,
    pub(crate) allowlist: BTreeMap<[u8; PublicKey::SIZE], ()>,
    pub(crate) owners: BTreeMap<[u8; PublicKey::SIZE], ()>,
}

impl StakeContract {
    pub const fn new() -> Self {
        Self {
            stakes: BTreeMap::new(),
            allowlist: BTreeMap::new(),
            owners: BTreeMap::new(),
        }
    }

    /// Gets a reference to a stake.
    pub fn get_stake(
        &self,
        key: &PublicKey,
    ) -> Option<impl Deref<Target = Stake> + '_> {
        self.stakes.get(&key.to_bytes())
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &PublicKey,
    ) -> Option<impl DerefMut<Target = Stake> + '_> {
        self.stakes.get_mut(&key.to_bytes())
    }

    /// Pushes the given `stake` onto the state for a given `public_key`. If a
    /// stake already exists for the given key, it is returned.
    pub fn insert_stake(
        &mut self,
        public_key: PublicKey,
        stake: Stake,
    ) -> Option<Stake> {
        self.stakes.insert(public_key.to_bytes(), stake)
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_mut_stake(
        &mut self,
        pk: &PublicKey,
    ) -> Option<impl DerefMut<Target = Stake> + '_> {
        let is_missing = self.stakes.get(&pk.to_bytes()).is_none();

        if is_missing {
            let stake = Stake::default();
            self.stakes.insert(pk.to_bytes(), stake);
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        self.stakes.get_mut(&pk.to_bytes())
    }

    /// Rewards a `public_key` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(
        &mut self,
        public_key: &PublicKey,
        value: u64,
    ) {
        let mut stake = self.load_mut_stake(public_key).expect("stake for any key exists");
        stake.increase_reward(value);
    }

    pub fn is_staked(
        &self,
        block_height: u64,
        key: &PublicKey,
    ) -> bool {
        let is_staked = self
            .stakes
            .get(&key.to_bytes())
            .filter(|s| s.is_valid(block_height))
            .is_some();
        is_staked
    }

    /// Gets a vector of all public keys and stakes.
    pub fn stakes(&self) -> Result<Vec<(PublicKey, Stake)>, Error> {
        let mut stakes: Vec<(PublicKey, Stake)> = Vec::new();

        // if let Some(branch) = self.stakes.first()? {
        //     for leaf in branch {
        //         let leaf = leaf?;
        //         stakes.push((leaf.key, leaf.val.clone()));
        //     }
        // }

        // todo: not sure if this is a correct translation of the above
        for entry in self.stakes.iter() {
            stakes.push((PublicKey::from_bytes(entry.0)?, entry.1.clone()));
        }

        Ok(stakes)
    }

    /// Gets a vector of all allowlisted keys.
    pub fn stakers_allowlist(&self) -> Result<Vec<PublicKey>, Error> {
        let mut stakes = Vec::new();

        // if let Some(branch) = self.allowlist.first()? {
        //     for leaf in branch {
        //         let leaf = leaf?;
        //         stakes.push(leaf.key);
        //     }
        // }

        // todo: not sure if this is a correct translation of the above
        for pk_bytes in self.allowlist.keys() {
            stakes.push(PublicKey::from_bytes(pk_bytes)?);
        }

        Ok(stakes)
    }

    /// Gets a vector of all owner keys.
    pub fn owners(&self) -> Result<Vec<PublicKey>, Error> {
        let mut stakes = Vec::new();

        // if let Some(branch) = self.owners.first()? {
        //     for leaf in branch {
        //         let leaf = leaf?;
        //         stakes.push(leaf.key);
        //     }
        // }

        // todo: not sure if this is a correct translation of the above
        for pk_bytes in self.owners.keys() {
            stakes.push(PublicKey::from_bytes(pk_bytes)?);
        }

        Ok(stakes)
    }

    pub fn allowlist_sign_message(counter: u64, staker: &PublicKey) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(u64::SIZE + PublicKey::SIZE);

        bytes.extend(counter.to_bytes());
        bytes.extend(staker.to_bytes());

        bytes
    }

    pub fn stake_sign_message(counter: u64, value: u64) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);

        bytes.extend(counter.to_bytes());
        bytes.extend(value.to_bytes());

        bytes
    }

    pub fn unstake_sign_message(counter: u64, note: Note) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(u64::SIZE + Note::SIZE);

        bytes.extend(counter.to_bytes());
        bytes.extend(note.to_bytes());

        bytes
    }

    pub fn withdraw_sign_message(
        counter: u64,
        address: StealthAddress,
        nonce: BlsScalar,
    ) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            u64::SIZE + StealthAddress::SIZE + BlsScalar::SIZE,
        );

        bytes.extend(counter.to_bytes());
        bytes.extend(address.to_bytes());
        bytes.extend(nonce.to_bytes());

        bytes
    }

    pub fn add_owner(&mut self, owner: PublicKey) {
        if !self.is_owner(&owner) {
            self.owners.insert(owner.to_bytes(), ());
        }
    }

    pub fn is_owner(&self, owner: &PublicKey) -> bool {
        self.owners.get(&owner.to_bytes()).is_some()
    }

    pub fn insert_allowlist(&mut self, staker: PublicKey) {
        if !self.is_allowlisted(&staker) {
            self.allowlist.insert(staker.to_bytes(), ());
        }
    }

    pub fn is_allowlisted(&self, staker: &PublicKey) -> bool {
        self.allowlist.get(&staker.to_bytes()).is_some()
    }
}
