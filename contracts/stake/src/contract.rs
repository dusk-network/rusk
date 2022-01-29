// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use canonical_derive::Canon;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use dusk_hamt::Map;
use microkelvin::First;
use phoenix_core::Note;

use alloc::vec::Vec;

#[cfg(feature = "transaction")]
mod transaction;

#[derive(Debug, Default, Clone, Canon)]
pub struct StakeContract {
    staked: Map<PublicKey, Stake>,
}

impl StakeContract {
    pub fn get_stake(&self, key: &PublicKey) -> Result<Stake, Error> {
        self.staked
            .get(key)?
            .map(|s| *s)
            .ok_or(Error::StakeNotFound)
    }

    pub fn push_stake(
        &mut self,
        pk: PublicKey,
        stake: Stake,
    ) -> Result<(), Error> {
        let exists = self.staked.get(&pk)?.is_some();
        if exists {
            return Err(Error::StakeAlreadyExists);
        }

        self.staked.insert(pk, stake)?;

        Ok(())
    }

    pub fn remove_stake(&mut self, pk: &PublicKey) -> Result<(), Error> {
        let removed = self.staked.remove(pk)?.is_some();
        if !removed {
            return Err(Error::StakeNotFound);
        }

        Ok(())
    }

    pub fn is_staked(
        &self,
        block_height: u64,
        key: &PublicKey,
    ) -> Result<bool, Error> {
        let is_staked = self
            .staked
            .get(key)?
            .filter(|s| s.is_valid(block_height))
            .is_some();

        Ok(is_staked)
    }

    /// Gets a vector of all public keys and stakes.
    pub fn stakes(&self) -> Result<Vec<(PublicKey, Stake)>, Error> {
        let mut stakes = Vec::new();

        if let Some(branch) = self.staked.first()? {
            for leaf in branch {
                let leaf = leaf?;
                stakes.push((leaf.key, leaf.val));
            }
        }

        Ok(stakes)
    }

    pub fn stake_sign_message(block_height: u64, stake: &Stake) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + Stake::SIZE);

        bytes.extend(&block_height.to_le_bytes());
        bytes.extend(stake.to_bytes());

        bytes
    }

    pub fn extend_sign_message(block_height: u64, stake: &Stake) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + Stake::SIZE);

        bytes.extend(&block_height.to_le_bytes());
        bytes.extend(stake.to_bytes());

        bytes
    }

    pub fn withdraw_sign_message(
        block_height: u64,
        stake: &Stake,
        note: &Note,
    ) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + Stake::SIZE + Note::SIZE);

        bytes.extend(&block_height.to_le_bytes());
        bytes.extend(stake.to_bytes());
        bytes.extend(note.to_bytes());

        bytes
    }
}
