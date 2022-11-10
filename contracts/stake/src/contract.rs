// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use dusk_pki::StealthAddress;
use microkelvin::First;
use phoenix_core::Note;

use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

#[cfg(feature = "transaction")]
mod transaction;

/// Contract keeping track of each public key's stake.
///
/// A caller can stake Dusk, and have it attached to a public key. This stake
/// has a maturation period, after which it is considered valid and the key
/// eligible to participate in the consensus.
///
/// Rewards may be received by a public key regardless of whether they have a
/// valid stake.
#[derive(Debug, Default, Clone, Canon)]
pub struct StakeContract {
    stakes: Map<PublicKey, Stake>,
    owners: Vec<PublicKey>,
}

impl StakeContract {
    /// Gets a reference to a stake.
    pub fn get_stake(
        &self,
        key: &PublicKey,
    ) -> Result<Option<impl Deref<Target = Stake> + '_>, Error> {
        Ok(self.stakes.get(key)?)
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(
        &mut self,
        key: &PublicKey,
    ) -> Result<Option<impl DerefMut<Target = Stake> + '_>, Error> {
        Ok(self.stakes.get_mut(key)?)
    }

    /// Pushes the given `stake` onto the state for a given `public_key`. If a
    /// stake already exists for the given key, it is returned.
    pub fn insert_stake(
        &mut self,
        public_key: PublicKey,
        stake: Stake,
    ) -> Result<Option<Stake>, Error> {
        Ok(self.stakes.insert(public_key, stake)?)
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_or_create_mut_stake(
        &mut self,
        pk: &PublicKey,
    ) -> Result<impl DerefMut<Target = Stake> + '_, Error> {
        let is_missing = self.stakes.get(pk)?.is_none();

        if is_missing {
            let stake = Stake::default();
            self.stakes.insert(*pk, stake)?;
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        Ok(self.stakes.get_mut(pk)?.unwrap())
    }

    /// Gets a mutable reference to the stake of a given key.
    #[allow(unused)]
    pub(crate) fn load_mut_stake(
        &mut self,
        pk: &PublicKey,
    ) -> Result<Option<impl DerefMut<Target = Stake> + '_>, Error> {
        let stake = self.stakes.get_mut(pk)?;
        Ok(stake)
    }

    /// Rewards a `public_key` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(
        &mut self,
        public_key: &PublicKey,
        value: u64,
    ) -> Result<(), Error> {
        let mut stake = self.load_or_create_mut_stake(public_key)?;
        stake.increase_reward(value);
        Ok(())
    }

    pub fn is_staked(
        &self,
        block_height: u64,
        key: &PublicKey,
    ) -> Result<bool, Error> {
        let is_staked = self
            .stakes
            .get(key)?
            .filter(|s| s.is_valid(block_height))
            .is_some();

        Ok(is_staked)
    }

    /// Gets a vector of all public keys and stakes.
    pub fn stakes(&self) -> Result<Vec<(PublicKey, Stake)>, Error> {
        let mut stakes = Vec::new();

        if let Some(branch) = self.stakes.first()? {
            for leaf in branch {
                let leaf = leaf?;
                stakes.push((leaf.key, leaf.val.clone()));
            }
        }

        Ok(stakes)
    }

    /// Gets a vector of all allowlisted keys.
    pub fn stakers_allowlist(&self) -> Result<Vec<PublicKey>, Error> {
        let mut stakes = Vec::new();

        if let Some(branch) = self.stakes.first()? {
            for leaf in branch {
                let leaf = leaf?;
                stakes.push(leaf.key);
            }
        }

        Ok(stakes)
    }

    /// Gets a vector of all owner keys.
    pub fn owners(&self) -> &Vec<PublicKey> {
        &self.owners
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

    pub fn add_owner(&mut self, owner: PublicKey) -> Result<(), Error> {
        if !self.owners.contains(&owner) {
            self.owners.push(owner);
        }
        Ok(())
    }

    pub fn is_owner(&self, owner: &PublicKey) -> Result<bool, Error> {
        Ok(self.owners.contains(owner))
    }

    pub fn insert_allowlist(&mut self, staker: PublicKey) -> Result<(), Error> {
        if !self.is_allowlisted(&staker)? {
            let stake = Stake::default();
            self.stakes.insert(staker, stake)?;
        }
        Ok(())
    }

    pub fn is_allowlisted(&self, staker: &PublicKey) -> Result<bool, Error> {
        Ok(self.stakes.get(staker)?.is_some())
    }
}
